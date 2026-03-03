use hdk::prelude::*;
use posts_integrity::*;
#[hdk_extern]
pub fn get_all_posts(_: ()) -> ExternResult<Vec<Link>> {
    let path = Path::from("all_posts");
    get_links(
        LinkQuery::try_new(path.path_entry_hash()?, LinkTypes::AllPosts)?,
        GetStrategy::Network,
    )
}
