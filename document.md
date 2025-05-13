# 项目进展
已完成所有基本功能的开发并通过本地测试。

设计文档如下：
# 设计文档
## 项目选择
Rust实现的Git客户端架构设计文档
## 小组成员
221220013 王泳智
## git init
一个完整的.git 文件夹至少包括：
1. HEAD 文件
2. objects/ 子文件夹
3. refs/ 子文件夹

`Repository::init(path)`将在 path 下创建创建.git 文件夹和上述子文件/文件夹
1. .git 文件夹已经存在，报错退出
2. .git 文件夹不存在，可以执行

init 命令调用这个方法，使用当前工作目录作为参数初始化文件夹

Repository 结构：
1. dir 表示.git 文件夹的路径
2. work_dir: 表示当前工作目录

常量：
1. OBJECTS_DIR: "objects/",将其与 dir 拼接得到objects存放文件夹
2. REFS_DIR: "refs/",同上
3. HEAD_FILE: "HEAD", 同上
4. GIT_DIR: ".git" 仅在 init 时使用，用于和 work_dir 拼接而成 dir

`Repository::is_vaild_git_dir(dir)`判断dir指向的文件夹是否是一个完整的.git 文件夹

`Repository::open(dir)`判断dir下是否存在一个完整的 .git 文件夹，然后 dir 为基础创建一个 repository 结构



## 对象结构
### Object trait
一个对象文件表示为"{type} {size}\0{contents}"

type: 字符串，表示对象类型，有三种取值
1. "blob"
2. "tree"
3. "commit"

size: 字符串，十进制表示的 contents 长度

contents：内容字节流

trait 方法：
1. sha1() 获取 20 个 bytes 数组的 sha1（sha1 serialize 的内容而来）
2. encoded_sha1() 将 sha1 编码为字符串
3. serialize() 将object 转换为字节序列，遵守上面的格式约定
### Blob
从一个文件中构建，contents 是文件的内容
从一个字节流中构建
### Tree
一个map，将文件名映射到文件对应 object 的 sha1 和 object 的 type (blob 或 tree)

保存的 contents：
{object type} {object sha1} {filename}

子目录会被映射为另一个tree 对象

可能的 tree 对象 content 如下：（暂时不考虑储存文件模式）
```
blob a906cb2a4a904a152e80877d4088654daad0c859      README
blob 8f94139338f9404f26296befa88755fc2598c289      Rakefile
tree 99f1a6d12cb4b6f19c8655fca46c3ecf317074e0      lib
```

write-tree: 将当前的index内容创建并同步到一组 tree 文件中
### Commit
存储 commit 所需的信息

提交对象的格式很简单：它先指定一个顶层树对象，代表当前项目快照； 然后是可能存在的父提交（前面描述的提交对象并不存在任何父提交）； 之后是作者/提交者信息（依据你的 user.name 和 user.email 配置来设定，外加一个时间戳）； 留空一行，最后是提交注释。

1. commit-tree 指定一个 tree 对象的 sha1 和 可能存在的 父亲 提交，创建一个 commit 对象并存储入对象数据库，返回此 commit 对象的 sha1
## ObjectDB 对象数据库操作
给定一个 path，可以基于这个 path 创建一个 ObjectDB 结构

ObjectDB 仅支持两种操作
1. 将一个实现了 Object trait 的对象存入 OBJECT DIR；使用 encoded sha1 确定存储位置和文件名，前两位作为子文件夹名，后18位作为文件名
2. 给定 sha1， 从 OBJECT DIR 取出对应的文件，返回字节流
   
## index

https://git-scm.com/docs/gitformat-index

index 是 repository 的一个**快照**。它的每个条目代表一个与 repository 目录下的文件，其内容相比此文件同步或更落后。

初始为空。只有 update-index 可以修改 index 的内容

index 存储的内容是一个个条目（entry）（以有序map实现）：

条目：文件相对 repository 的**相对路径** 文件对应的 blob 的sha1

条目中不含子文件夹名，只含文件名。以'/'作为路径分隔符。

接口：
增删查改条目

update-index(files) 命令指定了index需要与 repository 同步的文件。

如果 INDEX 文件不存在，则首先创建此文件。再从 INDEX 文件中创建 Index 结构。
1. file存在且 在 index 中没有对应条目，则创建并存储 blob 文件并添加对应条目。（添加文件）
2. file存在且在 index 中有对应条目。若其对应blob 文件不存在则创建并存储blob 文件并更新对应条目。否则不做任何事。（更新文件）
3. file 不存在且在index 中有对应条目。则删除对应条目（删除文件）
4. file 不存在且在 index 中没有对应条目。 错误，报错并退出。
   
最后将 Index 结构写回磁盘。

write-tree 命令可以将 index 转储为一组 tree 对象，创建并存储对应的文件。返回 repository 对应的 tree 对象的 sha1。此过程相当地简单和直接，因为 ObjectDB不会额外存储一个已经存在的对象，所以无需判断是否存在。

write-tree 在内部使用递归实现。即指定一个前缀（prefix），代表写入表示子目录 \<prefix\> 的树对象。如果write tree 写入的子目录内不存在其他子目录，则创建一个 tree 对象并返回其 sha1。否则，递归将子目录转储成一组 tree 对象。

## branch
branch 存储在 `.git/refs/heads` 文件夹下。文件名为 branch 名，文件内容是以字符串表示的 branch 引用的 commit 对象的哈希值。

如 文件名：`add-comments`，内容:`5cb4608b594abb69cfc8cddd0010ff6891099be2`，表示这是一个 branch，名字为 `add-comments`，引用 `5cb46`的 commit

branch 并不属于 git object，所以其自身提供了 save 和 load 方法，给定 path 用于保存和加载自身。

## HEAD
HEAD 可以存在于 `.git` 下或者 `.git/refs/remotes/<remote name>`下。

HEAD 保存一个 branch 文件相对 git dir 的相对路径。文件名为：`HEAD`，内容示例：`ref: refs/heads/add-comments`

HEAD 不属于 git object，所以其自身提供了 save 和 load 方法，给定 path 用于保存和加载自身。

## git add
接受一个 Vec 的文件名参数，对其中的每个文件调用 update-index

## git commit
接受一个 message 作为参数，调用 write-index 将 index 写入成一组 tree object。再利用 root tree 的sha1 调用 commit-tree 创建一个 commit。将 commit 的 sha 存入 HEAD
1. 如果 message 为空，报错并退出
2. 如果 write-index 得到的 root tree 的 sha1 和 HEAD的一致，则报错退出，认为 nothing has been staged
3. HEAD 分为 detached 和 ref 两种情况。存入 HEAD 时需要区分

## git checkout 
接受一个 tree obj 的 sha 作为参数，实现分为两步：
1. read-tree https://git-scm.com/docs/git-read-tree
   1. 将 `tree-sha` 提供的 tree 信息读取到 index 中，但不会实际更新它所“缓存”的任何文件。
2. checkout-index 将 index 中列出的所有文件复制到工作目录（不覆盖现有文件）。If a working file is untracked in the current branch and would be overwritten by the checkout, print There is an untracked file in the way; delete it, or add and commit it first. and exit; perform this check before doing anything else. 
3. 将 HEAD 设定为 checkout 的 branch

## git branch
接受一个name为参数。如果 name 代表的 branch 已经存在，报错退出。否则以当前commit 创建分支。

## git merge
找到当前 commit 与 merge 目标 branch commit 的 LCA。以 LCA 为基础 merge。

merge 逻辑如下：

https://sp21.datastructur.es/materials/proj/proj2/proj2#merge

> Any files that have been modified in the given branch since the split point, but not modified in the current branch since the split point should be changed to their versions in the given branch (checked out from the commit at the front of the given branch). These files should then all be automatically staged. To clarify, if a file is “modified in the given branch since the split point” this means the version of the file as it exists in the commit at the front of the given branch has different content from the version of the file at the split point. Remember: blobs are content addressable!

> Any files that have been modified in the current branch but not in the given branch since the split point should stay as they are.

> Any files that have been modified in both the current and given branch in the same way (i.e., both files now have the same content or were both removed) are left unchanged by the merge. If a file was removed from both the current and given branch, but a file of the same name is present in the working directory, it is left alone and continues to be absent (not tracked nor staged) in the merge.

> Any files that were not present at the split point and are present only in the current branch should remain as they are.

> Any files that were not present at the split point and are present only in the given branch should be checked out and staged.

> Any files present at the split point, unmodified in the current branch, and absent in the given branch should be removed (and untracked).

> Any files present at the split point, unmodified in the given branch, and absent in the current branch should remain absent.

> Any files modified in different ways in the current and given branches are in conflict. “Modified in different ways” can mean that the contents of both are changed and different from other, or the contents of one are changed and the other file is deleted, or the file was absent at the split point and has different contents in the given and current branches. In this case, replace the contents of the conflicted file with

## git rm
调用操作系统 API 删除文件后使用 git add 更新 index

## 特色功能
本 git 系统采用 plumbing + porcelain 的组织形式。

### Porcelain 面向用户命令
`commit`,`add`,`init`,`merge`等命令属于 porcelain 命令，提供用户友好的高级命令接口。

porcelain 命令是对 plumbing 命令的封装。组合一系列 plumbing 命令实现自身的功能，有利于逻辑解耦和代码复用。

### Plumbing 面向数据库命令
提供直接面向数据库的命令，一般不会由用户直接使用。

如：`write-tree`,`diff-index`,`update-index`,`read-tree`等。`add`可以由`update-tree`封装实现，`commit`可以由`diff-index`+`write-tree`封装实现。由此复用各个模块，避免重复代码编写。

