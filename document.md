# Rust实现的Git客户端架构设计文档
## 文件结构
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
## refs

## remote
此 git 系统的服务端只需要是一个支持 http GET/POST 操作的服务器。不需要专门的服务端。

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
2. checkout-index 将 index 中列出的所有文件复制到工作目录（不覆盖现有文件）。
3. 将 HEAD 设定为 checkout 的 branch

## git branch
接受一个name为参数。如果 name 代表的 branch 已经存在，报错退出。否则以当前commit 创建分支。

## git merge


## 一、核心架构设计
### 1.1 Repository中枢结构
```rust
#[derive(Clone)]
pub struct Repository {
    path: PathBuf,
    config: Config,
    object_db: Arc<dyn ObjectDatabase>,
    index: IndexManager,
}

impl Repository {
    /// 初始化仓库（对应git init）
    pub fn init(path: &Path) -> Result<Self, RepositoryError> {
        let git_dir = path.join(".git");
        create_dir_all(git_dir.join("objects"))?;
        create_dir_all(git_dir.join("refs/heads"))?;
        // 初始化config文件（参考网页1的权限控制设计）
        let config = Config::default().with_safe_permissions();
        Ok(Self { path, config, ... })
    }

    /// 打开已有仓库（基于网页1的Gix实现）
    pub fn open(path: &Path) -> Result<Self, RepositoryError> {
        let git_dir = find_git_dir(path)?; // 支持子目录查找
        let config = Config::parse_from(git_dir)?;
        Ok(Self {
            path: git_dir,
            config,
            object_db: ObjectDatabase::new(git_dir.join("objects")),
            index: IndexManager::new(git_dir.join("index")),
        })
    }
}
```


### 1.2 分层模块设计
```text
src/
├── plumbing/    # 底层命令实现
│   ├── objects/ # 对象操作（blob/tree/commit/tag）
│   ├── refs/    # 引用操作
│   └── index/   # 索引管理（基于网页1的高效迭代器设计）
├── porcelain/   # 用户友好命令
│   ├── init.rs
│   ├── add.rs
│   └── commit.rs
└── repository.rs # 核心结构实现
```

## 二、Plumbing层实现
### 2.1 对象数据库抽象
```rust
pub trait ObjectDatabase: Send + Sync {
    /// 按哈希获取对象（支持网页1的零成本抽象）
    fn get(&self, hash: &Hash) -> Result<GitObject, ObjectError>;
    
    /// 写入对象（包含自动压缩）
    fn put(&self, obj: GitObject) -> Result<Hash, ObjectError>;
}

/// 实现内存安全的对象解析（基于网页1的安全设计）
struct FileObjectDatabase {
    path: PathBuf,
    compression_level: u32,
}

impl ObjectDatabase for FileObjectDatabase {
    // 实现具体IO操作（包含网页1提及的权限检查）
}
```


### 2.2 典型Plumbing命令示例
**hash-object命令实现：**
```rust
pub fn hash_object(repo: &Repository, path: &Path) -> Result<Hash> {
    let content = std::fs::read(path)?;
    let blob = GitObject::Blob(content);
    repo.object_db.put(blob)
}
```

**update-index命令实现：**
```rust
pub fn update_index(repo: &mut Repository, entries: Vec<IndexEntry>) {
    repo.index.lock().update_entries(entries);
    repo.index.flush_to_disk()?; // 基于网页1的高效内存管理
}
```

## 三、Porcelain层实现
### 3.1 命令分发机制
```rust
impl Repository {
    pub fn execute_command(&mut self, cmd: Command) -> Result<String> {
        match cmd {
            Command::Porcelain(cmd) => self.handle_porcelain(cmd),
            Command::Plumbing(cmd) => self.handle_plumbing(cmd),
        }
    }

    fn handle_porcelain(&mut self, cmd: PorcelainCmd) -> Result<String> {
        match cmd {
            PorcelainCmd::Add { paths } => self.add(paths),
            PorcelainCmd::Commit { message } => self.commit(message),
            // ...其他命令
        }
    }
}
```

### 3.2 完整提交流程实现
```rust
impl Repository {
    pub fn commit(&mut self, message: &str) -> Result<Hash> {
        // 1. 创建树对象
        let tree = self.create_tree_from_index()?;
        
        // 2. 获取父提交（参考网页1的commit.peel_to_commit()实现）
        let parent = self.resolve_head()?;
        
        // 3. 生成提交对象
        let commit = CommitObject {
            tree: tree.hash(),
            parents: vec![parent],
            author: self.config.user.clone(),
            message: message.into(),
            timestamp: SystemTime::now(),
        };
        
        // 4. 写入对象数据库
        let hash = self.object_db.put(GitObject::Commit(commit))?;
        
        // 5. 更新HEAD引用（类似网页1的repo.head()处理）
        self.update_head_ref(hash)
    }
}
```


## 四、安全与性能设计
### 4.1 内存安全保障
• 所有文件操作使用`std::fs::OpenOptions`严格限制权限
• 使用`Arc<Mutex<...>>`管理共享状态（基于网页1的并发处理优势）
• 对象解析时进行完整性校验

### 4.2 性能优化策略
• 索引文件采用mmap内存映射（实现网页1的高效内存使用）
• 对象数据库使用LRU缓存
• 批量操作时启用并行处理（利用Rust的rayon库）

## 五、扩展接口设计
```rust
/// 可扩展的插件系统
pub trait Plugin {
    fn pre_command(&self, cmd: &Command) -> Result<()>;
    fn post_command(&self, cmd: &Command, result: &Result<String>) -> Result<()>;
}

/// 示例：钩子扩展
struct HookPlugin {
    hooks_dir: PathBuf,
}

impl Plugin for HookPlugin {
    fn pre_command(&self, cmd: &Command) -> Result<()> {
        // 执行pre-commit等钩子脚本
    }
}
```

## 六、开发路线建议
1. 先实现`init/add/commit`核心命令链
2. 补充`log/diff`等常用查询命令
3. 增加远程仓库支持（参考网页1的异步克隆实现）
4. 实现分支合并等高级功能

## 七、测试策略
```rust
#[cfg(test)]
mod tests {
    #[test]
    fn test_commit_flow() {
        let repo = Repository::init(temp_dir()).unwrap();
        repo.add(vec!["README.md"]).unwrap();
        let hash = repo.commit("initial commit").unwrap();
        assert!(hash.is_valid());
    }
}
```

---
该文档基于网页1中Gix项目的设计理念，结合传统Git的架构特点，实现了以下创新：
1. 统一Repository结构管理所有子系统
2. 严格区分plumbing/porcelain层实现
3. 类型安全的API设计（受益于Rust特性）
4. 可扩展的插件架构

建议结合[Gix源码](https://github.com/yourgixrepo)进行对照学习，实际开发时可使用`cargo doc --open`生成完整的API文档。