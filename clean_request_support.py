import re

with open('/home/philip/githome/robot-task-board/expanded.rs', 'r') as f:
    lines = f.readlines()

request_support = "".join(lines[83:1313])

# Remove #[automatically_derived] ... }
request_support = re.sub(r'#\[automatically_derived\]\s*impl.*?\{.*?(?=\n\s*(?:#\[|pub struct|pub enum|impl|mod|fn))\n?', '', request_support, flags=re.DOTALL)

# Let's just grab the struct / enum definitions and drop the impls that use unstable core!
# Actually, it's easier to just strip lines with `::core::` completely, but that might break code.

# To be perfectly safe, I will just manually provide a clean stub for request_support.
