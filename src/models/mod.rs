pub mod bookmark;
pub mod comment;
pub mod follow;
pub mod forum;
pub mod notification;
pub mod post;
pub mod post_tag;
pub mod refresh_token;
pub mod report;
pub mod tag;
pub mod user;
pub mod vote;

pub use bookmark::Entity as Bookmark;
pub use comment::{Entity as Comment, Model as CommentModel};
pub use follow::Entity as Follow;
pub use forum::{Entity as Forum, Model as ForumModel};
pub use notification::{Entity as Notification, Model as NotificationModel};
pub use post::{Entity as Post, Model as PostModel};
#[allow(unused_imports)]
pub use post_tag::Entity as PostTag;
#[allow(unused_imports)]
pub use refresh_token::Entity as RefreshToken;
pub use report::{Entity as Report, Model as ReportModel};
pub use tag::{Entity as Tag, Model as TagModel};
pub use user::{Entity as User, Model as UserModel};
pub use vote::{Entity as Vote, Model as VoteModel};
