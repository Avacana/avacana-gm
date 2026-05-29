use crate::git_manager::core::operations_advanced_support::{
    empty_advanced_result, map_advanced_error, normalize_non_empty,
    resolve_repository_relative_path,
};
use crate::git_manager::core::{AdvancedResult, GitResult};
use git2::{AttrCheckFlags, AttrValue, Repository};
use std::path::Path;

pub(crate) fn execute_query_attribute_operation(
    repository: &Repository,
    requested_path: &Path,
    attribute_name: &str,
) -> GitResult<AdvancedResult> {
    let attribute_name = normalize_non_empty(attribute_name, "advanced.query_attribute.name")?;
    let repository_relative_path = resolve_repository_relative_path(
        repository,
        requested_path,
        "advanced.query_attribute.path",
    )?;

    let attribute_value = repository
        .get_attr(
            repository_relative_path.as_path(),
            attribute_name,
            AttrCheckFlags::FILE_THEN_INDEX,
        )
        .map_err(|error| {
            map_advanced_error(
                &error,
                format!(
                    "advanced.query_attribute failed for `{}` and attribute `{attribute_name}`",
                    repository_relative_path.display()
                ),
            )
        })?;

    let mut result = empty_advanced_result();
    result
        .items
        .push(format!("path:{}", repository_relative_path.display()));
    result.items.push(format!("attribute:{attribute_name}"));

    match AttrValue::from_string(attribute_value) {
        AttrValue::True => {
            result.items.push("state:true".to_string());
            result.summary = Some(format!(
                "attribute `{attribute_name}` is set for `{}`",
                repository_relative_path.display()
            ));
        }
        AttrValue::False => {
            result.items.push("state:false".to_string());
            result.summary = Some(format!(
                "attribute `{attribute_name}` is unset for `{}`",
                repository_relative_path.display()
            ));
        }
        AttrValue::String(value) => {
            result.items.push("state:value".to_string());
            result.items.push(format!("value:{value}"));
            result.summary = Some(format!(
                "attribute `{attribute_name}` resolved to `{value}` for `{}`",
                repository_relative_path.display()
            ));
        }
        AttrValue::Bytes(value) => {
            let value = String::from_utf8_lossy(value).into_owned();
            result.items.push("state:value-bytes".to_string());
            result.items.push(format!("value:{value}"));
            result.summary = Some(format!(
                "attribute `{attribute_name}` resolved to bytes value for `{}`",
                repository_relative_path.display()
            ));
        }
        AttrValue::Unspecified => {
            result.items.push("state:unspecified".to_string());
            result.summary = Some(format!(
                "attribute `{attribute_name}` is unspecified for `{}`",
                repository_relative_path.display()
            ));
        }
    }

    Ok(result)
}

pub(crate) fn execute_check_ignore_operation(
    repository: &Repository,
    requested_path: &Path,
) -> GitResult<AdvancedResult> {
    let repository_relative_path =
        resolve_repository_relative_path(repository, requested_path, "advanced.check_ignore.path")?;

    let ignored = repository
        .is_path_ignored(repository_relative_path.as_path())
        .map_err(|error| {
            map_advanced_error(
                &error,
                format!(
                    "advanced.check_ignore failed for `{}`",
                    repository_relative_path.display()
                ),
            )
        })?;

    let mut result = empty_advanced_result();
    result
        .items
        .push(format!("path:{}", repository_relative_path.display()));
    result.items.push(format!("ignored:{ignored}"));
    result.summary = Some(if ignored {
        format!(
            "path `{}` is ignored by repository rules",
            repository_relative_path.display()
        )
    } else {
        format!(
            "path `{}` is not ignored by repository rules",
            repository_relative_path.display()
        )
    });

    Ok(result)
}
