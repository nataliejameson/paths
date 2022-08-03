use crate::AbsoluteJoinError;
use crate::AbsolutePathBufNewError;
use crate::AbsolutePathNewError;
use crate::JoinedAbsolute;
use crate::NormalizationFailed;
use crate::NotAbsolute;
use crate::RelativePath;
use crate::WasNotNormalized;
use ref_cast::RefCast;
use std::ops::Deref;
use std::path::Path;
use std::path::PathBuf;
use std::str::FromStr;

/// An absolute path. This must be normalized to begin with.
#[derive(Debug, Eq, PartialEq, Hash, Ord, PartialOrd, RefCast)]
#[cfg_attr(feature = "diesel", derive(diesel::expression::AsExpression))]
#[cfg_attr(feature="diesel", diesel(sql_type = diesel::sql_types::Text, not_sized))]
#[repr(transparent)]
pub struct AbsolutePath(Path);

impl AbsolutePath {
    /// Try to create a new [`AbsolutePath`], failing if the path provided is not absolute, or is not normalized.
    pub fn try_new<P: AsRef<Path> + ?Sized>(path: &P) -> Result<&Self, AbsolutePathNewError> {
        let p = path.as_ref();
        if p.is_relative() {
            Err(NotAbsolute(p.display().to_string()).into())
        } else {
            for c in p.components() {
                if c.as_os_str() == "." || c.as_os_str() == ".." {
                    return Err(WasNotNormalized(p.display().to_string()).into());
                }
            }
            Ok(Self::ref_cast(path.as_ref()))
        }
    }

    pub(crate) fn new_unchecked<P: AsRef<Path> + ?Sized>(path: &P) -> &Self {
        Self::try_new(path).expect("an absolute path")
    }

    /// Get a reference to the internal Path object.
    pub fn as_path(&self) -> &Path {
        &self.0
    }

    /// Attempt to join to a path.
    ///
    /// The provided path must be relative, and not traverse beyond the root of the filesystem.
    pub fn join<P: AsRef<Path>>(&self, path: P) -> Result<AbsolutePathBuf, AbsoluteJoinError> {
        let p = path.as_ref();
        if p.is_absolute() {
            Err(JoinedAbsolute(self.0.display().to_string(), p.display().to_string()).into())
        } else {
            AbsolutePathBuf::try_new(self.0.join(p)).map_err(|e| match e {
                AbsolutePathBufNewError::NormalizationFailed(e) => {
                    AbsoluteJoinError::NormalizationFailed(e)
                }
                _ => unreachable!(),
            })
        }
    }

    /// Attempt to join to a known relative path.
    ///
    /// This can only fail if the provided path attempts to traverse beyond the filesystem root.
    pub fn join_relative(
        &self,
        path: &RelativePath,
    ) -> Result<AbsolutePathBuf, NormalizationFailed> {
        AbsolutePathBuf::try_new(self.0.join(path.as_ref())).map_err(|e| match e {
            AbsolutePathBufNewError::NormalizationFailed(e) => e,
            _ => unreachable!(),
        })
    }

    pub fn parent(&self) -> Option<&AbsolutePath> {
        self.0.parent().map(AbsolutePath::new_unchecked)
    }
}

impl AsRef<Path> for AbsolutePath {
    fn as_ref(&self) -> &Path {
        self.as_path()
    }
}

impl Deref for AbsolutePath {
    type Target = Path;

    fn deref(&self) -> &Self::Target {
        self.as_path()
    }
}

#[cfg(feature = "serde")]
impl serde::Serialize for AbsolutePath {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        self.0.serialize(serializer)
    }
}

/// The "owned" analog for [`AbsolutePath`]. This attempts to normalize the path on instantiation.
#[derive(Debug, Eq, PartialEq, Hash, Clone, Ord, PartialOrd)]
#[cfg_attr(
    feature = "diesel",
    derive(diesel::expression::AsExpression, diesel::FromSqlRow)
)]
#[cfg_attr(feature="diesel", diesel(sql_type = diesel::sql_types::Text))]
pub struct AbsolutePathBuf(PathBuf);

impl AbsolutePathBuf {
    /// Attempt to create an instance of [`AbsolutePathBuf`].
    ///
    /// This will fail if the provided path is relative, or if, when normalizing, the path would
    /// traverse beyond the root of the filesystem.
    pub fn try_new<P: Into<PathBuf> + ?Sized>(path: P) -> Result<Self, AbsolutePathBufNewError> {
        let p = path.into();
        if p.is_relative() {
            Err(NotAbsolute(p.display().to_string()).into())
        } else {
            let needs_normalization = p
                .components()
                .any(|c| c.as_os_str() == "." || c.as_os_str() == "..");
            if !needs_normalization {
                Ok(Self(p))
            } else {
                let mut new_pb = Vec::with_capacity(p.components().count());
                for c in p.components() {
                    match c.as_os_str() {
                        x if x == "." => {}
                        x if x == ".." => {
                            if new_pb.pop().is_none() {
                                return Err(NormalizationFailed(p.display().to_string()).into());
                            }
                        }
                        x => {
                            new_pb.push(x);
                        }
                    }
                }
                if new_pb.is_empty() {
                    Err(NormalizationFailed(p.display().to_string()).into())
                } else {
                    Ok(Self(PathBuf::from_iter(new_pb)))
                }
            }
        }
    }

    #[allow(unused)]
    pub(crate) fn new_unchecked<P: Into<PathBuf> + ?Sized>(path: P) -> Self {
        Self::try_new(path).expect("an absolute path")
    }

    /// Get a reference to the internal Path object.
    pub fn as_path(&self) -> &Path {
        self.0.as_path()
    }

    /// Get a new [`AbsolutePath`] referencing the internal Path object.
    pub fn as_absolute_path(&self) -> &AbsolutePath {
        AbsolutePath::new_unchecked(self.0.as_path())
    }

    /// Attempt to join to a path.
    ///
    /// The provided path must be relative, and not traverse beyond the root of the filesystem.
    pub fn join<P: AsRef<Path> + ?Sized>(&self, path: &P) -> Result<Self, AbsoluteJoinError> {
        let p = path.as_ref();
        if p.is_absolute() {
            Err(JoinedAbsolute(self.0.display().to_string(), p.display().to_string()).into())
        } else {
            Self::try_new(self.0.join(path.as_ref())).map_err(|e| match e {
                AbsolutePathBufNewError::NormalizationFailed(e) => e.into(),
                AbsolutePathBufNewError::NotAbsolute(_) => std::unreachable!(),
            })
        }
    }

    /// Attempt to join to a known relative path.
    ///
    /// This can only fail if the provided path attempts to traverse beyond the filesystem root.
    pub fn join_relative(&self, path: &RelativePath) -> Result<Self, NormalizationFailed> {
        Self::try_new(self.0.join(path.as_ref())).map_err(|e| match e {
            AbsolutePathBufNewError::NormalizationFailed(e) => e,
            _ => std::unreachable!(),
        })
    }

    pub fn parent(&self) -> Option<&AbsolutePath> {
        self.0.parent().map(AbsolutePath::new_unchecked)
    }
}

impl From<&AbsolutePath> for AbsolutePathBuf {
    fn from(ap: &AbsolutePath) -> Self {
        AbsolutePathBuf::new_unchecked(&ap.0)
    }
}

impl AsRef<Path> for AbsolutePathBuf {
    fn as_ref(&self) -> &Path {
        self.as_path()
    }
}

impl AsRef<AbsolutePath> for AbsolutePathBuf {
    fn as_ref(&self) -> &AbsolutePath {
        AbsolutePath::new_unchecked(&self.0)
    }
}

impl Deref for AbsolutePathBuf {
    type Target = AbsolutePath;

    fn deref(&self) -> &Self::Target {
        AbsolutePath::new_unchecked(&self.0)
    }
}

impl FromStr for AbsolutePathBuf {
    type Err = AbsolutePathBufNewError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        AbsolutePathBuf::try_new(s)
    }
}

#[cfg(feature = "serde")]
impl serde::Serialize for AbsolutePathBuf {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        self.0.serialize(serializer)
    }
}

#[cfg(feature = "serde")]
impl<'de> serde::Deserialize<'de> for AbsolutePathBuf {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        use serde::de::Error;
        let path = PathBuf::deserialize(deserializer)?;
        AbsolutePathBuf::try_new(path).map_err(|e| D::Error::custom(format!("{}", e)))
    }
}

#[cfg(feature = "diesel")]
impl<DB> diesel::serialize::ToSql<diesel::sql_types::Text, DB> for AbsolutePath
where
    DB: diesel::backend::Backend,
    str: diesel::serialize::ToSql<diesel::sql_types::Text, DB>,
{
    fn to_sql<'b>(
        &'b self,
        out: &mut diesel::serialize::Output<'b, '_, DB>,
    ) -> diesel::serialize::Result {
        self.0.to_str().expect("paths should be utf8").to_sql(out)
    }
}

#[cfg(feature = "diesel")]
impl<DB> diesel::serialize::ToSql<diesel::sql_types::Text, DB> for AbsolutePathBuf
where
    DB: diesel::backend::Backend,
    str: diesel::serialize::ToSql<diesel::sql_types::Text, DB>,
{
    fn to_sql<'b>(
        &'b self,
        out: &mut diesel::serialize::Output<'b, '_, DB>,
    ) -> diesel::serialize::Result {
        self.0.to_str().expect("paths should be utf8").to_sql(out)
    }
}

#[cfg(feature = "diesel")]
impl<DB> diesel::deserialize::FromSql<diesel::sql_types::Text, DB> for AbsolutePathBuf
where
    DB: diesel::backend::Backend,
    String: diesel::deserialize::FromSql<diesel::sql_types::Text, DB>,
{
    fn from_sql(bytes: diesel::backend::RawValue<DB>) -> diesel::deserialize::Result<Self> {
        String::from_sql(bytes).and_then(|s| Ok(AbsolutePathBuf::try_new(s)?))
    }
}

#[cfg(test)]
mod test {
    use crate::AbsoluteJoinError;
    use crate::AbsolutePath;
    use crate::AbsolutePathBuf;
    use crate::AbsolutePathBufNewError;
    use crate::AbsolutePathNewError;
    use crate::JoinedAbsolute;
    use crate::NormalizationFailed;
    use crate::NotAbsolute;
    use crate::WasNotNormalized;
    use std::path::Path;

    #[test]
    fn path_try_new() -> anyhow::Result<()> {
        let cwd = std::env::current_dir()?;
        assert_eq!(
            cwd.join("foo.txt").as_path(),
            AbsolutePath::try_new(cwd.join("foo.txt").as_path())?.as_path()
        );

        assert_eq!(
            AbsolutePathNewError::NotAbsolute(NotAbsolute(String::from("foo.txt"))),
            AbsolutePath::try_new("foo.txt").unwrap_err()
        );
        assert_eq!(
            AbsolutePathNewError::WasNotNormalized(WasNotNormalized(
                cwd.join("foo/../../bar.txt").display().to_string()
            )),
            AbsolutePath::try_new(cwd.join("foo/../../bar.txt").as_path()).unwrap_err()
        );

        Ok(())
    }

    #[test]
    fn path_join() -> anyhow::Result<()> {
        let cwd = std::env::current_dir()?;
        let foo_bar = cwd.join("foo/bar");

        let original = AbsolutePath::try_new(foo_bar.as_path())?;
        assert_eq!(
            cwd.join("foo/bar/baz").as_path(),
            original.join("baz")?.as_path()
        );
        assert_eq!(
            cwd.join("foo/baz").as_path(),
            original.join("../baz")?.as_path()
        );
        assert_eq!(
            cwd.join("foo/bar/baz").as_path(),
            original.join("./baz")?.as_path()
        );
        assert_eq!(
            AbsoluteJoinError::JoinedAbsolute(JoinedAbsolute(
                original.as_ref().display().to_string(),
                cwd.as_path().display().to_string()
            )),
            original.join(cwd.as_path()).unwrap_err()
        );

        let back_to_root = "../".repeat(cwd.components().count() + 1);
        let root = original.join(&back_to_root)?;
        assert!(root.is_absolute());
        assert_eq!(Path::new("/"), root.as_path());

        let back_past_root = "../".repeat(cwd.components().count() + 2);

        assert_eq!(
            AbsoluteJoinError::NormalizationFailed(NormalizationFailed(
                cwd.join("foo/bar")
                    .join(&back_past_root)
                    .display()
                    .to_string()
            )),
            original.join(&back_past_root).unwrap_err()
        );

        Ok(())
    }

    #[test]
    fn path_parent() -> anyhow::Result<()> {
        let cwd = std::env::current_dir()?;
        let root = Path::new("/");
        let abs_root_buf = AbsolutePathBuf::try_new("/")?;

        let abs_cwd = AbsolutePath::try_new(&cwd)?;
        let abs_root = AbsolutePath::try_new(&abs_root_buf)?;

        assert!(cwd.parent().is_some());
        assert_eq!(
            AbsolutePath::try_new(cwd.parent().unwrap())?,
            abs_cwd.parent().unwrap()
        );
        assert!(root.parent().is_none());
        assert!(abs_root.parent().is_none());
        Ok(())
    }

    #[test]
    fn path_buf_try_new() -> anyhow::Result<()> {
        let cwd = std::env::current_dir()?;
        assert_eq!(
            cwd.join("foo.txt").as_path(),
            AbsolutePathBuf::try_new(cwd.join("foo.txt").as_path())?.as_path()
        );
        assert_eq!(
            cwd.join("foo/bar/quz.txt").as_path(),
            AbsolutePathBuf::try_new(cwd.join("foo/bar/baz/../quz.txt").as_path())?.as_path()
        );
        assert_eq!(
            cwd.join("foo/bar/baz/quz.txt").as_path(),
            AbsolutePathBuf::try_new(cwd.join("./foo/bar/baz/./quz.txt").as_path())?.as_path()
        );

        assert_eq!(
            AbsolutePathBufNewError::NotAbsolute(NotAbsolute(String::from("foo.txt"))),
            AbsolutePathBuf::try_new("foo.txt").unwrap_err()
        );

        let parent_dirs = "../".repeat(cwd.components().count());
        let past_root_path = cwd.join("foo").join(parent_dirs).join("../../bar.txt");
        assert_eq!(
            AbsolutePathBufNewError::NormalizationFailed(NormalizationFailed(
                past_root_path.display().to_string()
            )),
            AbsolutePathBuf::try_new(past_root_path.as_path()).unwrap_err()
        );

        Ok(())
    }

    #[test]
    fn path_buf_join() -> anyhow::Result<()> {
        let cwd = std::env::current_dir()?;
        let foo_bar = cwd.join("foo/bar");

        let original = AbsolutePathBuf::try_new(foo_bar.as_path())?;
        assert_eq!(
            cwd.join("foo/bar/baz").as_path(),
            original.join("baz")?.as_path()
        );
        assert_eq!(
            cwd.join("foo/baz").as_path(),
            original.join("../baz")?.as_path()
        );
        assert_eq!(
            cwd.join("foo/bar/baz").as_path(),
            original.join("./baz")?.as_path()
        );
        assert_eq!(
            AbsoluteJoinError::JoinedAbsolute(JoinedAbsolute(
                original.as_absolute_path().display().to_string(),
                cwd.as_path().display().to_string()
            )),
            original.join(cwd.as_path()).unwrap_err()
        );

        let back_to_root = "../".repeat(cwd.components().count() + 1);
        let root = original.join(&back_to_root)?;
        assert!(root.is_absolute());
        assert_eq!(Path::new("/"), root.as_path());

        let back_past_root = "../".repeat(cwd.components().count() + 2);

        assert_eq!(
            AbsoluteJoinError::NormalizationFailed(NormalizationFailed(
                cwd.join("foo/bar")
                    .join(&back_past_root)
                    .display()
                    .to_string()
            )),
            original.join(&back_past_root).unwrap_err()
        );

        Ok(())
    }

    #[test]
    fn path_buf_parent() -> anyhow::Result<()> {
        let cwd = std::env::current_dir()?;
        let root = Path::new("/");

        let abs_cwd = AbsolutePathBuf::try_new(&cwd)?;
        let abs_root = AbsolutePathBuf::try_new(&root)?;

        assert!(cwd.parent().is_some());
        assert_eq!(
            AbsolutePath::try_new(cwd.parent().unwrap())?,
            abs_cwd.parent().unwrap()
        );
        assert!(root.parent().is_none());
        assert!(abs_root.parent().is_none());
        Ok(())
    }
}

#[cfg(all(test, feature = "serde"))]
mod serde_tests {
    use crate::AbsolutePath;
    use crate::AbsolutePathBuf;

    #[test]
    fn path_serializes() -> anyhow::Result<()> {
        let cwd = std::env::current_dir()?;
        let p = AbsolutePath::try_new(&cwd)?;
        assert_eq!(format!("\"{}\"", cwd.display()), serde_json::to_string(&p)?);
        Ok(())
    }

    #[test]
    fn path_buf_serializes() -> anyhow::Result<()> {
        let cwd = std::env::current_dir()?;
        let p = AbsolutePathBuf::try_new(&cwd)?;
        assert_eq!(format!("\"{}\"", cwd.display()), serde_json::to_string(&p)?);
        Ok(())
    }

    #[test]
    fn path_buf_deserializes() -> anyhow::Result<()> {
        let cwd = std::env::current_dir()?;
        let serialized_good = format!("\"{}/foo/./bar/../baz\"", cwd.display());
        let serialized_relative = "\"foo/bar\"".to_owned();
        let serialized_traversal = format!(
            "\"{}\"",
            cwd.join("../".repeat(cwd.components().count())).display()
        );

        let expected = AbsolutePathBuf::try_new(&cwd.join("foo/baz"))?;
        assert_eq!(
            expected,
            serde_json::from_str::<AbsolutePathBuf>(&serialized_good)?
        );
        assert!(serde_json::from_str::<AbsolutePathBuf>(&serialized_relative).is_err());
        assert!(serde_json::from_str::<AbsolutePathBuf>(&serialized_traversal).is_err());
        Ok(())
    }
}

#[cfg(all(test, feature = "diesel"))]
mod test_diesel {
    use crate::diesel::QueryDsl;
    use crate::diesel_helpers::create_table;
    use crate::diesel_helpers::insert_values;
    use crate::AbsolutePath;
    use crate::AbsolutePathBuf;
    use diesel::RunQueryDsl;

    #[derive(Queryable, Insertable, Clone, Debug, Eq, PartialEq)]
    #[diesel(table_name = crate::diesel_helpers::schema::test_files)]
    struct TestFile {
        id: i32,
        x: AbsolutePathBuf,
        y: Option<AbsolutePathBuf>,
    }

    #[derive(Insertable, Clone, Debug, Eq, PartialEq)]
    #[diesel(table_name = crate::diesel_helpers::schema::test_files)]
    struct TestFileLog<'a> {
        id: i32,
        x: &'a AbsolutePath,
        y: Option<&'a AbsolutePath>,
    }

    #[test]
    fn path_to_sql() -> anyhow::Result<()> {
        let cwd = std::env::current_dir()?;
        let abs_foo_bar = cwd.join("foo/bar.txt");
        let abs_bar_baz = cwd.join("bar/baz.txt");

        let mut connection = create_table()?;

        use crate::diesel_helpers::schema::test_files::dsl::*;

        let expected = vec![
            TestFile {
                id: 1,
                x: AbsolutePathBuf::try_new(&abs_foo_bar)?,
                y: None,
            },
            TestFile {
                id: 2,
                x: AbsolutePathBuf::try_new(&abs_foo_bar)?,
                y: Some(AbsolutePathBuf::try_new(&abs_bar_baz)?),
            },
        ];

        diesel::insert_into(test_files)
            .values(vec![
                &TestFileLog {
                    id: 1,
                    x: AbsolutePath::try_new(&abs_foo_bar)?,
                    y: None,
                },
                &TestFileLog {
                    id: 2,
                    x: AbsolutePath::try_new(&abs_foo_bar)?,
                    y: Some(AbsolutePath::try_new(&abs_bar_baz)?),
                },
            ])
            .execute(&mut connection)?;

        let rows = test_files.load::<TestFile>(&mut connection)?;
        assert_eq!(expected, rows);

        Ok(())
    }

    #[test]
    fn path_buf_to_sql() -> anyhow::Result<()> {
        let cwd = std::env::current_dir()?;
        let abs_foo_bar = cwd.join("foo/bar.txt");
        let abs_bar_baz = cwd.join("bar/baz.txt");

        let mut connection = create_table()?;

        use crate::diesel_helpers::schema::test_files::dsl::*;

        let expected = vec![
            TestFile {
                id: 1,
                x: AbsolutePathBuf::try_new(&abs_foo_bar)?,
                y: None,
            },
            TestFile {
                id: 2,
                x: AbsolutePathBuf::try_new(&abs_foo_bar)?,
                y: Some(AbsolutePathBuf::try_new(&abs_bar_baz)?),
            },
        ];

        diesel::insert_into(test_files)
            .values(vec![
                &TestFile {
                    id: 1,
                    x: AbsolutePathBuf::try_new(&abs_foo_bar)?,
                    y: None,
                },
                &TestFile {
                    id: 2,
                    x: AbsolutePathBuf::try_new(&abs_foo_bar)?,
                    y: Some(AbsolutePathBuf::try_new(&abs_bar_baz)?),
                },
            ])
            .execute(&mut connection)?;

        let rows = test_files.load::<TestFile>(&mut connection)?;
        assert_eq!(expected, rows);

        Ok(())
    }

    #[test]
    fn path_buf_from_sql() -> anyhow::Result<()> {
        let cwd = std::env::current_dir()?;
        let abs_foo_bar = cwd.join("foo/bar.txt");
        let abs_bar_baz = cwd.join("bar/baz.txt");
        let abs_foo_bar_str = abs_foo_bar.display().to_string();
        let abs_bar_baz_str = abs_bar_baz.display().to_string();

        let mut connection = create_table()?;

        insert_values(
            &mut connection,
            &[
                (1, &abs_foo_bar_str, None),
                (2, "foo/bar.txt", None),
                (3, &abs_foo_bar_str, Some(&abs_bar_baz_str)),
                (4, &abs_foo_bar_str, Some("bar/baz.txt")),
            ],
        )?;

        use crate::diesel_helpers::schema::test_files::dsl::*;

        let expected = [
            TestFile {
                id: 1,
                x: AbsolutePathBuf::try_new(&abs_foo_bar)?,
                y: None,
            },
            TestFile {
                id: 3,
                x: AbsolutePathBuf::try_new(&abs_foo_bar)?,
                y: Some(AbsolutePathBuf::try_new(&abs_bar_baz)?),
            },
        ];

        assert_eq!(expected[0], test_files.find(1).first(&mut connection)?);
        assert!(test_files
            .find(2)
            .first::<TestFile>(&mut connection)
            .is_err());
        assert_eq!(expected[1], test_files.find(3).first(&mut connection)?);
        assert!(test_files
            .find(4)
            .first::<TestFile>(&mut connection)
            .is_err());

        Ok(())
    }
}
