use crate::errors::JoinedAbsolute;
use crate::errors::NotRelative;
use crate::AbsolutePath;
use crate::AbsolutePathBuf;
use crate::NormalizationFailed;
use gazebo::dupe::Dupe;
use std::ops::Deref;
use std::path::Path;
use std::path::PathBuf;

/// A relative path. This is not normalized until joined to an absolute path.
#[derive(Debug, Eq, PartialEq, Hash, Copy, Clone, Dupe)]
pub struct RelativePath<'a>(&'a Path);

impl<'a> RelativePath<'a> {
    /// Attempt to create an instance of [`RelativePath`].
    ///
    /// This will fail if the provided path is absolute.
    pub fn try_new<P: AsRef<Path> + ?Sized + 'a>(path: &'a P) -> Result<Self, NotRelative> {
        let p = path.as_ref();
        if p.is_absolute() {
            Err(NotRelative(p.display().to_string()))
        } else {
            Ok(Self(path.as_ref()))
        }
    }

    pub(crate) fn new_unchecked<P: AsRef<Path> + ?Sized + 'a>(path: &'a P) -> Self {
        Self::try_new(path).expect("an absolute path")
    }

    /// Get a reference to the internal Path object.
    pub fn as_path(&self) -> &Path {
        self.0
    }

    /// Attempt to join to a path.
    ///
    /// The provided path must be relative.
    pub fn join<P: AsRef<Path>>(&self, path: P) -> Result<RelativePathBuf, JoinedAbsolute> {
        let p = path.as_ref();
        if p.is_absolute() {
            Err(JoinedAbsolute(
                self.0.display().to_string(),
                p.display().to_string(),
            ))
        } else {
            Ok(RelativePathBuf::try_new(self.0.join(p))
                .expect("Already verified both pieces are relative"))
        }
    }

    /// Join this to an [`AbsolutePath`], normalizing the joined path.
    ///
    /// This can only fail the normalization causes traversal beyond the filesystem root.
    pub fn try_into_absolute(
        &self,
        abs: &AbsolutePath,
    ) -> Result<AbsolutePathBuf, NormalizationFailed> {
        abs.join_relative(self)
    }
}

impl<'a> AsRef<Path> for RelativePath<'a> {
    fn as_ref(&self) -> &Path {
        self.as_path()
    }
}

impl<'a> Deref for RelativePath<'a> {
    type Target = Path;

    fn deref(&self) -> &Self::Target {
        self.as_path()
    }
}

/// The "owned" analog for [`RelativePath`]. This is not normalized until joined to an absolute path.
#[derive(Debug, Eq, PartialEq, Hash, Clone)]
pub struct RelativePathBuf(PathBuf);

impl RelativePathBuf {
    /// Attempt to create an instance of [`RelativePathBuf`].
    ///
    /// This will fail if the provided path is absolute.
    pub fn try_new<P: Into<PathBuf> + ?Sized>(path: P) -> Result<Self, NotRelative> {
        let p = path.into();
        if p.is_absolute() {
            Err(NotRelative(p.display().to_string()))
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
                                new_pb.push(x);
                            }
                        }
                        x => {
                            new_pb.push(x);
                        }
                    }
                }

                Ok(Self(PathBuf::from_iter(new_pb)))
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

    /// Get a new [`RelativePath`] referencing the internal Path object.
    pub fn as_relative_path(&self) -> RelativePath {
        RelativePath::new_unchecked(self.0.as_path())
    }

    /// Attempt to join to a path.
    ///
    /// The provided path must be relative.
    pub fn join<P: AsRef<Path> + ?Sized>(&self, path: &P) -> Result<Self, JoinedAbsolute> {
        let p = path.as_ref();
        if p.is_absolute() {
            Err(JoinedAbsolute(
                self.0.display().to_string(),
                p.display().to_string(),
            ))
        } else {
            Ok(Self::try_new(self.0.join(p)).expect("Already verified both pieces were relative"))
        }
    }

    /// Join this to an [`AbsolutePath`], normalizing the joined path.
    ///
    /// This can only fail the normalization causes traversal beyond the filesystem root.
    pub fn try_into_absolute(
        &self,
        abs: &AbsolutePath,
    ) -> Result<AbsolutePathBuf, NormalizationFailed> {
        abs.join_relative(&self.as_relative_path())
    }
}

impl AsRef<Path> for RelativePathBuf {
    fn as_ref(&self) -> &Path {
        self.as_path()
    }
}

impl Deref for RelativePathBuf {
    type Target = Path;

    fn deref(&self) -> &Self::Target {
        self.as_path()
    }
}

#[cfg(test)]
mod test {
    use crate::AbsolutePath;
    use crate::AbsolutePathBuf;
    use crate::JoinedAbsolute;
    use crate::NotRelative;
    use crate::RelativePath;
    use crate::RelativePathBuf;
    use std::path::Path;

    #[test]
    fn path_try_new() -> anyhow::Result<()> {
        let cwd = std::env::current_dir()?;

        assert_eq!(
            Path::new("foo.txt"),
            RelativePath::try_new("foo.txt")?.as_path()
        );
        assert_eq!(
            Path::new("foo/../bar/../../baz/./quz.txt"),
            RelativePath::try_new("foo/../bar/../../baz/./quz.txt")?.as_path()
        );

        assert_eq!(
            Err(NotRelative(cwd.join("foo.txt").display().to_string())),
            RelativePath::try_new(cwd.join("foo.txt").as_path())
        );
        Ok(())
    }

    #[test]
    fn path_join() -> anyhow::Result<()> {
        let cwd = std::env::current_dir()?;

        assert_eq!(
            Path::new("foo/bar"),
            RelativePath::try_new("foo")?.join("bar")?.as_path()
        );
        assert_eq!(
            Path::new("../baz/quz.txt"),
            RelativePath::try_new("foo")?
                .join("../bar/../../baz/./quz.txt")?
                .as_path()
        );

        assert_eq!(
            Err(JoinedAbsolute(
                "foo".to_owned(),
                cwd.join("foo.txt").display().to_string()
            )),
            RelativePath::try_new("foo")?.join(cwd.join("foo.txt"))
        );
        Ok(())
    }

    #[test]
    fn path_try_into_absolute() -> anyhow::Result<()> {
        let cwd = std::env::current_dir()?;
        let foo_bar = cwd.join("foo/bar");

        let original = AbsolutePath::try_new(foo_bar.as_path())?;

        assert_eq!(
            cwd.join("foo/bar/baz").as_path(),
            RelativePath::try_new("baz")?
                .try_into_absolute(&original)?
                .as_path()
        );
        assert_eq!(
            cwd.join("foo").as_path(),
            RelativePath::try_new("../")?
                .try_into_absolute(&original)?
                .as_path()
        );
        assert_eq!(
            cwd.join("foo/bar/baz/quz").as_path(),
            RelativePath::try_new("baz/./quz")?
                .try_into_absolute(&original)?
                .as_path()
        );

        Ok(())
    }

    #[test]
    fn path_buf_try_new() -> anyhow::Result<()> {
        let cwd = std::env::current_dir()?;
        assert_eq!(
            Path::new("foo.txt"),
            RelativePathBuf::try_new("foo.txt")?.as_path()
        );
        assert_eq!(
            Path::new("../baz/quz.txt"),
            RelativePathBuf::try_new("foo/../bar/../../baz/./quz.txt")?.as_path()
        );

        assert_eq!(
            Err(NotRelative(cwd.join("foo.txt").display().to_string())),
            RelativePathBuf::try_new(cwd.join("foo.txt"))
        );

        Ok(())
    }

    #[test]
    fn path_buf_try_into_absolute() -> anyhow::Result<()> {
        let cwd = std::env::current_dir()?;

        let original = AbsolutePathBuf::try_new(cwd.join("foo/bar"))?;

        assert_eq!(
            cwd.join("foo/bar/baz").as_path(),
            RelativePathBuf::try_new("baz")?
                .try_into_absolute(&original.as_absolute_path())?
                .as_path()
        );
        assert_eq!(
            cwd.join("foo").as_path(),
            RelativePathBuf::try_new("../")?
                .try_into_absolute(&original.as_absolute_path())?
                .as_path()
        );
        assert_eq!(
            cwd.join("foo/bar/baz/quz").as_path(),
            RelativePathBuf::try_new("baz/./quz")?
                .try_into_absolute(&original.as_absolute_path())?
                .as_path()
        );

        Ok(())
    }
}
