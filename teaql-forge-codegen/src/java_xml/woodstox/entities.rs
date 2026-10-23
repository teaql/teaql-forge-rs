//! Diagnostic input streams retain each entity's own cursor and caller chain.
use super::{at, plain, woodstox_name_character, Failure};
use std::collections::BTreeMap;

#[derive(Clone, Copy, Default)]
struct Cursor { row: usize, column: usize, cr: bool }

impl Cursor {
    fn advance(&mut self, ch: char) {
        if ch == '\r' || ch == '\n' && !self.cr { self.row += 1; self.column = 0; }
        else if ch != '\n' || !self.cr { self.column += ch.len_utf16(); }
        self.cr = ch == '\r';
    }
}

struct Definition { value: String, start: Cursor }
struct Frame { name: String, parent: usize, caller: Cursor }
struct Mark { end: usize, cursor: Cursor, frame: usize }

pub(super) struct Stream {
    pub text: String,
    marks: Vec<Mark>,
    frames: Vec<Frame>,
    errors: Vec<(usize, Failure)>,
    definitions: BTreeMap<String, Definition>,
    count: usize,
}

impl Stream {
    pub fn internal(text: &str) -> Option<Self> {
        let start=text.find("<!DOCTYPE")?;
        let body=super::super::dtd::doctype_span(text,start).map(|(end,_)|end).unwrap_or(text.len());
        let header=&text[start..body];
        if header.contains("SYSTEM")||header.contains("PUBLIC")||header.contains('%') {return None;}
        let version11 = super::declaration(text).get("version").is_some_and(|version| version == "1.1");
        let mut stream=Self{text:String::new(),marks:Vec::new(),frames:vec![Frame{name:String::new(),parent:0,caller:Cursor::default()}],errors:Vec::new(),definitions:BTreeMap::new(),count:0};
        let mut scan=start;
        while scan<body {
            let rest=&text[scan..body];
            let close=if rest.starts_with("<!--"){Some("-->")}else if rest.starts_with("<?"){Some("?>")}else{None};
            if let Some(close)=close{scan+=rest.find(close).map(|n|n+close.len()).unwrap_or(rest.len());continue;}
            if !rest.starts_with("<!ENTITY"){scan+=rest.chars().next().unwrap().len_utf8();continue;}
            let begin=scan;
            let mut p=begin+8;
            while text[p..].starts_with([' ','\t','\r','\n']) {p+=1;}
            if p >= body { break; }
            let end=text[p..body].char_indices()
                .find(|(offset,ch)|!woodstox_name_character(*ch,*offset==0,version11))
                .map(|(offset,_)|p+offset).unwrap_or(body);
            let name=text[p..end].to_owned();p=end;
            while text[p..].starts_with([' ','\t','\r','\n']) {p+=1;}
            let Some(q)=text[p..].chars().next().filter(|c|matches!(c,'\''|'"'))else{scan=p;continue;};
            p+=1;let Some(end)=text[p..body].find(q).map(|n|n+p)else{break;};
            let mut cursor=Cursor{row:1,..Default::default()};for ch in text[..p].chars(){cursor.advance(ch);}
            let value=super::super::dtd::character_references(&text[p..end]);
            stream.definitions.entry(name).or_insert(Definition{value,start:cursor});scan=end+1;
        }
        let mut cursor=Cursor{row:1,..Default::default()};
        for ch in text[..body].chars(){cursor.advance(ch);stream.push(ch,cursor,0);}
        stream.expand(&text[body..],cursor,0,false);
        Some(stream)
    }

    fn push(&mut self,ch:char,cursor:Cursor,frame:usize){self.text.push(ch);self.marks.push(Mark{end:self.text.len(),cursor,frame});}
    fn deferred(&mut self,error:Failure,cursor:Cursor,frame:usize){self.errors.push((self.text.len()+1,error));self.push('\0',cursor,frame);}
    fn error(&self,cursor:Cursor,frame:usize,message:String)->Failure{
        let mut error=plain(format!("{message}\n at [row,col {{unknown-source}}]: [{},{}]",cursor.row,cursor.column));
        let mut f=frame;
        while f!=0 {let frame=&self.frames[f];error.message.push_str(&format!("\n from [row,col {{unknown-source}}]: [{},{}]",frame.caller.row,frame.caller.column));f=frame.parent;}
        error.line=Some(cursor.row);error
    }
    fn expand(&mut self,text:&str,mut cursor:Cursor,frame:usize,parent_attribute:bool){
        let mut pos=0;let mut tag=false;let mut quote=None;
        while pos<text.len(){
            let rest=&text[pos..];
            let close=if !tag&&rest.starts_with("<!--"){Some("-->")}else if !tag&&rest.starts_with("<![CDATA["){Some("]]>")}else if !tag&&rest.starts_with("<?"){Some("?>")}else{None};
            if let Some(close)=close{
                let end=rest.find(close).map(|n|pos+n+close.len()).unwrap_or(text.len());
                for ch in text[pos..end].chars(){cursor.advance(ch);self.push(ch,cursor,frame);}pos=end;continue;
            }
            let ch=rest.chars().next().unwrap();
            if ch=='&'{
                if let Some(end)=rest.find(';').map(|n|pos+n+1){
                    let name=&text[pos+1..end-1];
                    if let Some(definition)=self.definitions.get(name){
                        let value=definition.value.clone();let start=definition.start;
                        for c in text[pos..end].chars(){cursor.advance(c);}
                        let mut caller=cursor;caller.column+=1;
                        let mut active=frame;let mut depth=1;let mut recursive=false;
                        while active!=0{if self.frames[active].name==name{recursive=true;break;}active=self.frames[active].parent;depth+=1;}
                        if recursive{let error=self.error(cursor,frame,format!("Illegal entity expansion: entity \"{name}\" expands itself recursively."));self.deferred(error,cursor,frame);}
                        else if depth>500{self.deferred(plain("Maximum entity expansion depth limit (500) exceeded"),cursor,frame);}
                        else if self.count>=100000{self.deferred(plain("Maximum entity expansion count limit (100000) exceeded"),cursor,frame);}
                        else{self.count+=1;let id=self.frames.len();self.frames.push(Frame{name:name.into(),parent:frame,caller});self.expand(&value,start,id,parent_attribute||quote.is_some());}
                        pos=end;continue;
                    }
                }
            }
            cursor.advance(ch);self.push(ch,cursor,frame);
            if !parent_attribute{
                if let Some(q)=quote{if ch==q{quote=None;}}
                else if tag&&matches!(ch,'\''|'"'){quote=Some(ch);}
                else if ch=='<'{tag=true;}
                else if ch=='>'{tag=false;}
            }
            pos+=ch.len_utf8();
        }
    }
    fn mark(&self,pos:usize)->Option<&Mark>{let i=self.marks.partition_point(|m|m.end<pos);self.marks.get(i)}
    pub fn frame(&self,pos:usize)->usize{self.mark(pos+1).map(|m|m.frame).unwrap_or(0)}
    pub fn at(&self,pos:usize,message:impl Into<String>)->Failure{
        let mut error=if pos==0{at("",0,message.into())}else if let Some(mark)=self.mark(pos){self.error(mark.cursor,mark.frame,message.into())}else{at(&self.text,pos,message.into())};
        error.offset=Some(pos);error
    }
    pub fn finish(&self,result:Result<(),Failure>)->Result<(),Failure>{
        if let Err(error)=&result{
            if let Some((_,failure))=self.errors.iter().find(|(pos,_)|error.offset.is_some_and(|end|*pos<=end)){return Err(failure.clone());}
        }
        result
    }
}
