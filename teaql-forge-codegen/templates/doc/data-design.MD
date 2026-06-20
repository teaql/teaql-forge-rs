#  gateway Data Design Document

## Change History
| # |    Date    |        Author |
|:--|:----------:|--------------:|
| 1 | 2025-03-04 | TeaQL Doc Rob |
| 2 | 2025-03-04 | TeaQL Doc Rob |

## Summary of Concepts

@model.svg
<svg width="900" height="600"><g transform="translate(450,300)"><text style="font-size: 28px; font-family: Impact;" fill="#d1c55f" text-anchor="middle" transform="translate(1,-44)rotate(41.567337356628215)">Platform</text><text style="font-size: 24px; font-family: Impact;" fill="#9d8676" text-anchor="middle" transform="translate(-117,49)rotate(10.528828677950173)">Merchant</text><text style="font-size: 24px; font-family: Impact;" fill="#96c36a" text-anchor="middle" transform="translate(72,41)rotate(-57.599894927847046)">Service Request</text><text style="font-size: 20px; font-family: Impact;" fill="#ac0a58" text-anchor="middle" transform="translate(-202,-60)rotate(26.940367407271978)">Merchant Status</text><text style="font-size: 20px; font-family: Impact;" fill="#eb7972" text-anchor="middle" transform="translate(-205,-22)rotate(50.619254123005135)">User</text><text style="font-size: 20px; font-family: Impact;" fill="#d16a9e" text-anchor="middle" transform="translate(-89,-1)rotate(46.18282597975531)">Asset</text><text style="font-size: 20px; font-family: Impact;" fill="#9e1d5b" text-anchor="middle" transform="translate(109,-80)rotate(-48.51924483450105)">Request Type</text><text style="font-size: 16px; font-family: Impact;" fill="#fd5140" text-anchor="middle" transform="translate(135,82)rotate(33.9239011236715)">Good Merchant</text><text style="font-size: 16px; font-family: Impact;" fill="#e85857" text-anchor="middle" transform="translate(-4,55)rotate(-16.99509226960525)">Cylinder</text><text style="font-size: 16px; font-family: Impact;" fill="#fc0d97" text-anchor="middle" transform="translate(186,-141)rotate(11.125318338730565)">Good Cylinder</text><text style="font-size: 16px; font-family: Impact;" fill="#c1c852" text-anchor="middle" transform="translate(87,107)rotate(-34.10331285966841)">Dewar</text><text style="font-size: 16px; font-family: Impact;" fill="#d7853c" text-anchor="middle" transform="translate(-83,-98)rotate(-0.21273951700268867)">My View</text><text style="font-size: 16px; font-family: Impact;" fill="#ce5644" text-anchor="middle" transform="translate(6,-95)rotate(41.91531433481309)">Another View</text></g></svg>
@model.svg



## Tables Index


| # |  Table Name  |Columns Count |Relation Count |
|:-------------|:-----------------------|:-----------------------|:-----------------------|
 | 1 | [PLATFORM_DATA](#Table-PLATFORM_DATA) |3|3|
 | 2 | [MERCHANT_DATA](#Table-MERCHANT_DATA) |5|2|
 | 3 | [GOOD_MERCHANT_DATA](#Table-GOOD_MERCHANT_DATA) |2|0|
 | 4 | [MERCHANT_STATUS_DATA](#Table-MERCHANT_STATUS_DATA) |3|1|
 | 5 | [USER_DATA](#Table-USER_DATA) |2|1|
 | 6 | [CYLINDER_DATA](#Table-CYLINDER_DATA) |1|0|
 | 7 | [ASSET_DATA](#Table-ASSET_DATA) |4|1|
 | 8 | [GOOD_CYLINDER_DATA](#Table-GOOD_CYLINDER_DATA) |1|0|
 | 9 | [DEWAR_DATA](#Table-DEWAR_DATA) |2|0|
 | 10 | [MY_VIEW_DATA](#Table-MY_VIEW_DATA) |3|0|
 | 11 | [ANOTHER_VIEW_DATA](#Table-ANOTHER_VIEW_DATA) |3|0|
 | 12 | [SERVICE_REQUEST_DATA](#Table-SERVICE_REQUEST_DATA) |2|2|
 | 13 | [REQUEST_TYPE_DATA](#Table-REQUEST_TYPE_DATA) |3|1|


## Tables

 
### Table PLATFORM_DATA

#### Columns
| # | Column Name | Name    | Type        | Sample       |
|:--|:-----------|:--------|:------------|:-------------|
|1|ID|Id|BIGINT|1 |
|2|NAME|Name|VARCHAR(100)|好平台 |
|3|FOUNDED|Founded|TIMESTAMP|createTime() |
|4|CONTACT_NUMBER|Contact Number|VARCHAR(100)|121212-1212 |
|5|VERSION|Version|BIGINT|1 |



#### Index

| # | Index Name | Columns | Type | Remark       |
|:--|:----------|:----------------|----------------:|:-------------|
|1|PK_PLATFORM_ID|ID|PRIMARY KEY||
|2|PK_PLATFORM_ID_VERSION|ID,VERSION|UNIQUE KEY||
|3|IDX_PLATFORM_FOUNDED|FOUNDED|INDEX||




#### Constraints

| # | Constraint Name | Column | Reference Table | Reference Column       |Remark       |
|:--|:---------------|:----------------|:----------------|:-------------|:-------------|
|1|PK_PLATFORM_ID|ID|-|-|PRIMARY KEY|


#### Depended by

| # | Table Name | Reference Column | Remark       |
|:--|:----------|:-----------------|:-------------|
|1 |MERCHANT_DATA| PLATFORM|- |
|2 |MERCHANT_STATUS_DATA| PLATFORM|- |
|3 |USER_DATA| PLATFORM|- |






 
### Table MERCHANT_DATA

#### Columns
| # | Column Name | Name    | Type        | Sample       |
|:--|:-----------|:--------|:------------|:-------------|
|1|ID|Id|BIGINT|1 |
|2|NAME|Name|VARCHAR(100)|好商家 |
|3|CODE|Code|VARCHAR(100)|uniStr(prefix=MM) |
|4|OWNER|Owner|BIGINT|1 |
|5|STATUS|Status|BIGINT|1 |
|6|PLATFORM|Platform|BIGINT|1 |
|7|VERSION|Version|BIGINT|1 |



#### Index

| # | Index Name | Columns | Type | Remark       |
|:--|:----------|:----------------|----------------:|:-------------|
|1|PK_MERCHANT_ID|ID|PRIMARY KEY||
|2|PK_MERCHANT_ID_VERSION|ID,VERSION|UNIQUE KEY||
|3|IDX_MERCHANT_OWNER|OWNER|INDEX||
|4|IDX_MERCHANT_STATUS|STATUS|INDEX||
|5|IDX_MERCHANT_PLATFORM|PLATFORM|INDEX||




#### Constraints

| # | Constraint Name | Column | Reference Table | Reference Column       |Remark       |
|:--|:---------------|:----------------|:----------------|:-------------|:-------------|
|1|PK_MERCHANT_ID|ID|-|-|PRIMARY KEY|
|2|FK_MERCHANT_OWNER|OWNER|USER_DATA|ID|CASCADE ON DELETE and UPDATE|
|3|FK_MERCHANT_STATUS|STATUS|MERCHANT_STATUS_DATA|ID|CASCADE ON DELETE and UPDATE|
|4|FK_MERCHANT_PLATFORM|PLATFORM|PLATFORM_DATA|ID|CASCADE ON DELETE and UPDATE|


#### Depends Tables

| # | Table Name | Reference Column | Remark       |
|:--|:----------|:-----------------|:-------------|
|1 |USER_DATA| OWNER|- |
|2 |MERCHANT_STATUS_DATA| STATUS|- |
|3 |PLATFORM_DATA| PLATFORM|- |



#### Depended by

| # | Table Name | Reference Column | Remark       |
|:--|:----------|:-----------------|:-------------|
|1 |ASSET_DATA| MERCHANT|- |
|2 |ANOTHER_VIEW_DATA| MERCHANT|- |






 
### Table GOOD_MERCHANT_DATA

#### Columns
| # | Column Name | Name    | Type        | Sample       |
|:--|:-----------|:--------|:------------|:-------------|
|1|ID|Id|BIGINT|1 |
|2|GOOD_NAME|Good Name|text|text() |
|3|QUANTITY|Quantity|NUMERIC(19,7)|45.00 |



#### Index

| # | Index Name | Columns | Type | Remark       |
|:--|:----------|:----------------|----------------:|:-------------|
|1|PK_GOOD_MERCHANT_ID|ID|PRIMARY KEY||
|2|PK_GOOD_MERCHANT_ID_VERSION|ID,VERSION|UNIQUE KEY||




#### Constraints

| # | Constraint Name | Column | Reference Table | Reference Column       |Remark       |
|:--|:---------------|:----------------|:----------------|:-------------|:-------------|
|1|PK_GOOD_MERCHANT_ID|ID|-|-|PRIMARY KEY|





 
### Table MERCHANT_STATUS_DATA

#### Columns
| # | Column Name | Name    | Type        | Sample       |
|:--|:-----------|:--------|:------------|:-------------|
|1|ID|Id|BIGINT|1 |
|2|CODE|Code|VARCHAR(100)|open |
|3|NAME|Name|VARCHAR(100)|1 |
|4|PLATFORM|Platform|BIGINT|1 |
|5|VERSION|Version|BIGINT|1 |



#### Index

| # | Index Name | Columns | Type | Remark       |
|:--|:----------|:----------------|----------------:|:-------------|
|1|PK_MERCHANT_STATUS_ID|ID|PRIMARY KEY||
|2|PK_MERCHANT_STATUS_ID_VERSION|ID,VERSION|UNIQUE KEY||
|3|IDX_MERCHANT_STATUS_PLATFORM|PLATFORM|INDEX||




#### Constraints

| # | Constraint Name | Column | Reference Table | Reference Column       |Remark       |
|:--|:---------------|:----------------|:----------------|:-------------|:-------------|
|1|PK_MERCHANT_STATUS_ID|ID|-|-|PRIMARY KEY|
|2|FK_MERCHANT_STATUS_PLATFORM|PLATFORM|PLATFORM_DATA|ID|CASCADE ON DELETE and UPDATE|


#### Depends Tables

| # | Table Name | Reference Column | Remark       |
|:--|:----------|:-----------------|:-------------|
|1 |PLATFORM_DATA| PLATFORM|- |



#### Depended by

| # | Table Name | Reference Column | Remark       |
|:--|:----------|:-----------------|:-------------|
|1 |MERCHANT_DATA| STATUS|- |






#### Internal Data
|ID |CODE |NAME |PLATFORM |VERSION |
|:----------|:----------|:----------|:----------|:----------|
 
### Table USER_DATA

#### Columns
| # | Column Name | Name    | Type        | Sample       |
|:--|:-----------|:--------|:------------|:-------------|
|1|ID|Id|BIGINT|1 |
|2|NAME|Name|VARCHAR(100)|张三 |
|3|PLATFORM|Platform|BIGINT|1 |
|4|VERSION|Version|BIGINT|1 |



#### Index

| # | Index Name | Columns | Type | Remark       |
|:--|:----------|:----------------|----------------:|:-------------|
|1|PK_USER_ID|ID|PRIMARY KEY||
|2|PK_USER_ID_VERSION|ID,VERSION|UNIQUE KEY||
|3|IDX_USER_PLATFORM|PLATFORM|INDEX||




#### Constraints

| # | Constraint Name | Column | Reference Table | Reference Column       |Remark       |
|:--|:---------------|:----------------|:----------------|:-------------|:-------------|
|1|PK_USER_ID|ID|-|-|PRIMARY KEY|
|2|FK_USER_PLATFORM|PLATFORM|PLATFORM_DATA|ID|CASCADE ON DELETE and UPDATE|


#### Depends Tables

| # | Table Name | Reference Column | Remark       |
|:--|:----------|:-----------------|:-------------|
|1 |PLATFORM_DATA| PLATFORM|- |



#### Depended by

| # | Table Name | Reference Column | Remark       |
|:--|:----------|:-----------------|:-------------|
|1 |MERCHANT_DATA| OWNER|- |






 
### Table CYLINDER_DATA

#### Columns
| # | Column Name | Name    | Type        | Sample       |
|:--|:-----------|:--------|:------------|:-------------|
|1|ID|Id|BIGINT|1 |
|2|SIZE|Size|INTEGER|11234 |



#### Index

| # | Index Name | Columns | Type | Remark       |
|:--|:----------|:----------------|----------------:|:-------------|
|1|PK_CYLINDER_ID|ID|PRIMARY KEY||
|2|PK_CYLINDER_ID_VERSION|ID,VERSION|UNIQUE KEY||




#### Constraints

| # | Constraint Name | Column | Reference Table | Reference Column       |Remark       |
|:--|:---------------|:----------------|:----------------|:-------------|:-------------|
|1|PK_CYLINDER_ID|ID|-|-|PRIMARY KEY|





 
### Table ASSET_DATA

#### Columns
| # | Column Name | Name    | Type        | Sample       |
|:--|:-----------|:--------|:------------|:-------------|
|1|ID|Id|BIGINT|1 |
|2|NAME|Name|VARCHAR(100)|一个好东西 |
|3|BARCODE|Barcode|VARCHAR(100)|1231aa |
|4|PRICE|Price|NUMERIC(19,7)|1.5 |
|5|MERCHANT|Merchant|BIGINT|1 |
|6|VERSION|Version|BIGINT|1 |



#### Index

| # | Index Name | Columns | Type | Remark       |
|:--|:----------|:----------------|----------------:|:-------------|
|1|PK_ASSET_ID|ID|PRIMARY KEY||
|2|PK_ASSET_ID_VERSION|ID,VERSION|UNIQUE KEY||
|3|IDX_ASSET_MERCHANT|MERCHANT|INDEX||




#### Constraints

| # | Constraint Name | Column | Reference Table | Reference Column       |Remark       |
|:--|:---------------|:----------------|:----------------|:-------------|:-------------|
|1|PK_ASSET_ID|ID|-|-|PRIMARY KEY|
|2|FK_ASSET_MERCHANT|MERCHANT|MERCHANT_DATA|ID|CASCADE ON DELETE and UPDATE|


#### Depends Tables

| # | Table Name | Reference Column | Remark       |
|:--|:----------|:-----------------|:-------------|
|1 |MERCHANT_DATA| MERCHANT|- |



#### Depended by

| # | Table Name | Reference Column | Remark       |
|:--|:----------|:-----------------|:-------------|
|1 |MY_VIEW_DATA| ASSET|- |






 
### Table GOOD_CYLINDER_DATA

#### Columns
| # | Column Name | Name    | Type        | Sample       |
|:--|:-----------|:--------|:------------|:-------------|
|1|ID|Id|BIGINT|1 |
|2|LEVEL|Level|VARCHAR(100)|level1 |



#### Index

| # | Index Name | Columns | Type | Remark       |
|:--|:----------|:----------------|----------------:|:-------------|
|1|PK_GOOD_CYLINDER_ID|ID|PRIMARY KEY||
|2|PK_GOOD_CYLINDER_ID_VERSION|ID,VERSION|UNIQUE KEY||




#### Constraints

| # | Constraint Name | Column | Reference Table | Reference Column       |Remark       |
|:--|:---------------|:----------------|:----------------|:-------------|:-------------|
|1|PK_GOOD_CYLINDER_ID|ID|-|-|PRIMARY KEY|





 
### Table DEWAR_DATA

#### Columns
| # | Column Name | Name    | Type        | Sample       |
|:--|:-----------|:--------|:------------|:-------------|
|1|ID|Id|BIGINT|1 |
|2|WEIGHT|Weight|NUMERIC(19,7)|33.22 |
|3|THINNESS|Thinness|NUMERIC(10,2)|number(sqlType=NUMERIC(10 |



#### Index

| # | Index Name | Columns | Type | Remark       |
|:--|:----------|:----------------|----------------:|:-------------|
|1|PK_DEWAR_ID|ID|PRIMARY KEY||
|2|PK_DEWAR_ID_VERSION|ID,VERSION|UNIQUE KEY||




#### Constraints

| # | Constraint Name | Column | Reference Table | Reference Column       |Remark       |
|:--|:---------------|:----------------|:----------------|:-------------|:-------------|
|1|PK_DEWAR_ID|ID|-|-|PRIMARY KEY|





 
### Table MY_VIEW_DATA

#### Columns
| # | Column Name | Name    | Type        | Sample       |
|:--|:-----------|:--------|:------------|:-------------|
|1|ID|Id|BIGINT|1 |
|2|NAME|Name|VARCHAR(100)|aaaa |
|3|ASSET|Asset|BIGINT|1 |
|4|SERVICE_REQUEST|Service Request|BIGINT|1 |
|5|VERSION|Version|BIGINT|1 |



#### Index

| # | Index Name | Columns | Type | Remark       |
|:--|:----------|:----------------|----------------:|:-------------|
|1|PK_MY_VIEW_ID|ID|PRIMARY KEY||
|2|PK_MY_VIEW_ID_VERSION|ID,VERSION|UNIQUE KEY||
|3|IDX_MY_VIEW_ASSET|ASSET|INDEX||
|4|IDX_MY_VIEW_SERVICE_REQUEST|SERVICE_REQUEST|INDEX||




#### Constraints

| # | Constraint Name | Column | Reference Table | Reference Column       |Remark       |
|:--|:---------------|:----------------|:----------------|:-------------|:-------------|
|1|PK_MY_VIEW_ID|ID|-|-|PRIMARY KEY|
|2|FK_MY_VIEW_ASSET|ASSET|ASSET_DATA|ID|CASCADE ON DELETE and UPDATE|
|3|FK_MY_VIEW_SERVICE_REQUEST|SERVICE_REQUEST|SERVICE_REQUEST_DATA|ID|CASCADE ON DELETE and UPDATE|


#### Depends Tables

| # | Table Name | Reference Column | Remark       |
|:--|:----------|:-----------------|:-------------|
|1 |ASSET_DATA| ASSET|- |
|2 |SERVICE_REQUEST_DATA| SERVICE_REQUEST|- |






 
### Table ANOTHER_VIEW_DATA

#### Columns
| # | Column Name | Name    | Type        | Sample       |
|:--|:-----------|:--------|:------------|:-------------|
|1|ID|Id|BIGINT|1 |
|2|NAME|Name|VARCHAR(100)|aaaa |
|3|MERCHANT|Merchant|BIGINT|1 |
|4|SERVICE_REQUEST|Service Request|BIGINT|1 |
|5|VERSION|Version|BIGINT|1 |



#### Index

| # | Index Name | Columns | Type | Remark       |
|:--|:----------|:----------------|----------------:|:-------------|
|1|PK_ANOTHER_VIEW_ID|ID|PRIMARY KEY||
|2|PK_ANOTHER_VIEW_ID_VERSION|ID,VERSION|UNIQUE KEY||
|3|IDX_ANOTHER_VIEW_MERCHANT|MERCHANT|INDEX||
|4|IDX_ANOTHER_VIEW_SERVICE_REQUEST|SERVICE_REQUEST|INDEX||




#### Constraints

| # | Constraint Name | Column | Reference Table | Reference Column       |Remark       |
|:--|:---------------|:----------------|:----------------|:-------------|:-------------|
|1|PK_ANOTHER_VIEW_ID|ID|-|-|PRIMARY KEY|
|2|FK_ANOTHER_VIEW_MERCHANT|MERCHANT|MERCHANT_DATA|ID|CASCADE ON DELETE and UPDATE|
|3|FK_ANOTHER_VIEW_SERVICE_REQUEST|SERVICE_REQUEST|SERVICE_REQUEST_DATA|ID|CASCADE ON DELETE and UPDATE|


#### Depends Tables

| # | Table Name | Reference Column | Remark       |
|:--|:----------|:-----------------|:-------------|
|1 |MERCHANT_DATA| MERCHANT|- |
|2 |SERVICE_REQUEST_DATA| SERVICE_REQUEST|- |






 
### Table SERVICE_REQUEST_DATA

#### Columns
| # | Column Name | Name    | Type        | Sample       |
|:--|:-----------|:--------|:------------|:-------------|
|1|ID|Id|BIGINT|1 |
|2|ACTION_TYPE|Action Type|BIGINT|1 |
|3|SNAPSHOT|Snapshot|text|jsonMe() |
|4|VERSION|Version|BIGINT|1 |



#### Index

| # | Index Name | Columns | Type | Remark       |
|:--|:----------|:----------------|----------------:|:-------------|
|1|PK_SERVICE_REQUEST_ID|ID|PRIMARY KEY||
|2|PK_SERVICE_REQUEST_ID_VERSION|ID,VERSION|UNIQUE KEY||
|3|IDX_SERVICE_REQUEST_ACTION_TYPE|ACTION_TYPE|INDEX||




#### Constraints

| # | Constraint Name | Column | Reference Table | Reference Column       |Remark       |
|:--|:---------------|:----------------|:----------------|:-------------|:-------------|
|1|PK_SERVICE_REQUEST_ID|ID|-|-|PRIMARY KEY|
|2|FK_SERVICE_REQUEST_ACTION_TYPE|ACTION_TYPE|REQUEST_TYPE_DATA|ID|CASCADE ON DELETE and UPDATE|


#### Depends Tables

| # | Table Name | Reference Column | Remark       |
|:--|:----------|:-----------------|:-------------|
|1 |REQUEST_TYPE_DATA| ACTION_TYPE|- |



#### Depended by

| # | Table Name | Reference Column | Remark       |
|:--|:----------|:-----------------|:-------------|
|1 |MY_VIEW_DATA| SERVICE_REQUEST|- |
|2 |ANOTHER_VIEW_DATA| SERVICE_REQUEST|- |






 
### Table REQUEST_TYPE_DATA

#### Columns
| # | Column Name | Name    | Type        | Sample       |
|:--|:-----------|:--------|:------------|:-------------|
|1|ID|Id|BIGINT|1 |
|2|NAME|Name|VARCHAR(100)|场景1 |
|3|CODE|Code|VARCHAR(100)|SCENARIO1 |
|4|ENTRANCE_VIEW|Entrance View|VARCHAR(100)|my_view |
|5|VERSION|Version|BIGINT|1 |



#### Index

| # | Index Name | Columns | Type | Remark       |
|:--|:----------|:----------------|----------------:|:-------------|
|1|PK_REQUEST_TYPE_ID|ID|PRIMARY KEY||
|2|PK_REQUEST_TYPE_ID_VERSION|ID,VERSION|UNIQUE KEY||




#### Constraints

| # | Constraint Name | Column | Reference Table | Reference Column       |Remark       |
|:--|:---------------|:----------------|:----------------|:-------------|:-------------|
|1|PK_REQUEST_TYPE_ID|ID|-|-|PRIMARY KEY|


#### Depended by

| # | Table Name | Reference Column | Remark       |
|:--|:----------|:-----------------|:-------------|
|1 |SERVICE_REQUEST_DATA| ACTION_TYPE|- |






#### Internal Data
|ID |NAME |CODE |ENTRANCE_VIEW |VERSION |
|:----------|:----------|:----------|:----------|:----------|
|1|场景1|SCENARIO1|my_view|1|
|2|场景2|SCENARIO2|another_view|1|
|3|场景3|SCENARIO3|another_view|1|

