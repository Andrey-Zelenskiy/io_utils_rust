// Copyright Andrey Zelenskiy, 2024-2025

use std::io::Write;

use std::fs::{copy, create_dir_all, OpenOptions};

use std::path::{Path, PathBuf};

use serde::Deserialize;

/* ---------------------------- */
/* Project directory parameters */
/* ---------------------------- */
#[derive(Deserialize)]
pub struct ProjectManager {
    // Path to the project directory
    path: String,
    // Output file extension
    extension: String,
    // Type of behaviour if project files already exist
    overwrite_type: OverwriteType,
}

// Instructions for dealing with files that already exist
#[derive(Deserialize)]
pub enum OverwriteType {
    // Interrupts the program if duplicates are located
    Panic,
    // Copies duplicates to an archive folder
    Archive,
    // Overwrites all files
    Overwrite,
    // Ignore existing file during the writing
    Ignore,
}

impl ProjectManager {
    // Initialize output files
    pub fn initialize_output_files(
        &self,
        files: Vec<&mut FileManager>,
    ) -> Result<(), String> {
        files
            .into_iter()
            .map(|file| {
                file.set_project_path(&self.path)
                    .set_extension(&self.extension)
            })
            .try_for_each(|file| self.try_initialize_output(file))
    }

    // Attempts to initialize output files depending on the overwrite
    //  conditions
    fn try_initialize_output(
        &self,
        file: &mut FileManager,
    ) -> Result<(), String> {
        match &self.overwrite_type {
            OverwriteType::Panic => {
                if file.path().exists() {
                    Err(String::from(
                        "Permission denied to overwrite existing output files.",
                    ))
                } else {
                    file.initialize_output();
                    Ok(())
                }
            }
            OverwriteType::Archive => {
                // Create an archive directory
                if file.path().exists() {
                    let archive_path = Path::new(&self.path).join("archive");

                    if !archive_path.exists() {
                        if let Err(reason) = create_dir_all(&archive_path) {
                            panic!(
                                "Unable to create archive directory {:?}: {:?}",
                                archive_path,
                                reason.kind()
                            );
                        }
                    }
                    self.move_to_archive(&file.path(), &archive_path);
                }

                file.initialize_output();
                Ok(())
            }
            OverwriteType::Overwrite => {
                file.initialize_output();
                Ok(())
            }
            OverwriteType::Ignore => {
                if file.path().exists() {
                    file.to_writer(false);
                } else {
                    file.initialize_output();
                }
                Ok(())
            }
        }
    }

    // Move file to archive
    fn move_to_archive(&self, file_path: &Path, archive_path: &Path) {
        let filename = file_path.file_name().unwrap();
        let relative_path = file_path.parent().unwrap().file_name().unwrap();

        if !archive_path.join(relative_path).exists() {
            if let Err(reason) =
                create_dir_all(archive_path.join(relative_path))
            {
                panic!(
                    "Cannot create {:?} directory: {:?}",
                    archive_path.join(relative_path),
                    reason.kind()
                );
            }
        }

        if let Err(reason) =
            copy(file_path, archive_path.join(relative_path).join(filename))
        {
            panic!(
                "Cannot move file {:?} to {:?}: {:?}",
                file_path,
                archive_path.join(relative_path).join(filename),
                reason.kind()
            );
        }
    }
}

/* ---------------------- */
/* Output file parameters */
/* ---------------------- */

// Type for output file manipulation
#[derive(Clone, Debug, PartialEq)]
pub enum FileManager {
    Builder {
        // Column descriptions in the output file
        header: Option<String>,
        // Project path
        project_path: Option<String>,
        // Output path (relative to the project path)
        output_path: Option<String>,
        // File name
        name: Option<String>,
        // File extension
        extension: Option<String>,
    },
    Initializer {
        header: Option<String>,
        // Absolute path to the output file
        path: PathBuf,
    },
    Writer {
        path: PathBuf,
        // Permission for writting to the file
        writable: bool,
    },
}

impl Default for FileManager {
    fn default() -> Self {
        Self::Builder {
            header: None,
            project_path: None,
            output_path: None,
            name: None,
            extension: None,
        }
    }
}

impl FileManager {
    // Builder methods

    // Set the header
    pub fn set_header(&mut self, header_str: &str) -> &mut Self {
        if let Self::Builder { header, .. } = self {
            *header = Some(String::from(header_str));
        }
        self
    }

    // Add a path to the root project directory
    pub fn set_project_path(&mut self, project_path_str: &str) -> &mut Self {
        if let Self::Builder { project_path, .. } = self {
            *project_path = Some(String::from(project_path_str));
        }
        self
    }

    // Set the output directory path (relative to the project directory)
    pub fn set_output_path(&mut self, output_path_str: &str) -> &mut Self {
        if let Self::Builder { output_path, .. } = self {
            *output_path = Some(String::from(output_path_str));
        }
        self
    }

    // Change the name of the output file
    pub fn set_file_name(&mut self, name_str: &str) -> &mut Self {
        if let Self::Builder { name, .. } = self {
            *name = Some(String::from(name_str));
        }
        self
    }

    // Change the extension of the ouput file
    pub fn set_extension(&mut self, extension_str: &str) -> &mut Self {
        if let Self::Builder { extension, .. } = self {
            *extension = Some(String::from(extension_str));
        }
        self
    }

    // Obtain the path to the output file
    pub fn path(&self) -> PathBuf {
        match self {
            Self::Builder {
                project_path,
                output_path,
                name,
                extension,
                ..
            } => {
                let mut path = PathBuf::new();

                if let Some(s) = project_path {
                    path.push(s);
                }

                if let Some(s) = output_path {
                    path.push(s);
                }

                match name {
                    Some(s) => path.push(s),
                    None => path.push("file"),
                }

                let _ = match extension {
                    Some(s) => path.set_extension(s),
                    None => path.set_extension("dat"),
                };

                path
            }
            Self::Initializer { path, .. } => PathBuf::from(path),
            Self::Writer { path, .. } => PathBuf::from(path),
        }
    }

    // Initializer methods

    // Create the output file
    pub fn initialize_output(&mut self) {
        self.to_initializer();

        if let Self::Initializer { header, path } = self {
            // Create output directory
            if !path.parent().unwrap().exists() {
                if let Err(reason) = create_dir_all(path.parent().unwrap()) {
                    panic!(
                        "Cannot initialize output directory: {:?}",
                        reason.kind(),
                    );
                }
            }

            match OpenOptions::new()
                .create(true)
                .truncate(true)
                .write(true)
                .open(path.as_path())
            {
                Ok(mut file) => match header {
                    Some(header_str) => {
                        if let Err(reason) = writeln!(file, "{header_str}") {
                            panic!(
                                "Could not write to file {:?}: {:?}",
                                path,
                                reason.kind()
                            );
                        }
                    }
                    None => (),
                },
                Err(reason) => panic!(
                    "Could not open file {:?}: {:?}",
                    path,
                    reason.kind()
                ),
            }

            // Convert to Writer with write permissions
            self.to_writer(true);
        }
    }

    // Write methods
    pub fn writable(&self) -> bool {
        match self {
            Self::Builder { .. } | Self::Initializer { .. } => true,
            Self::Writer { writable, .. } => *writable,
        }
    }

    // State transitions

    // Function that can be used to return the current state of the object
    pub fn build(&mut self) -> Self {
        self.clone()
    }

    // If the current state is Builder change it to Initializer
    pub fn to_initializer(&mut self) {
        if let Self::Builder { header, .. } = self {
            *self = Self::Initializer {
                header: header.clone(),
                path: self.path(),
            }
        }
    }

    // If the current state is Builder or Initializer, change it to Writer
    pub fn to_writer(&mut self, writable: bool) {
        match self {
            Self::Builder { .. } | Self::Initializer { .. } => {
                // Attempt to create an absolute path for the output
                let mut path = self.path();

                path = match path.canonicalize() {
                    Ok(absolute_path) => absolute_path,
                    Err(_) => path,
                };

                *self = Self::Writer { path, writable }
            }
            Self::Writer { .. } => (),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::fs::{remove_dir_all, remove_file};

    use super::*;

    #[test]
    fn overwrite_files() {
        // Setup test project directory tree
        let project_manager = ProjectManager {
            path: "test_overwrite".to_owned(),
            extension: "dat".to_owned(),
            overwrite_type: OverwriteType::Overwrite,
        };

        let mut test_file_1 = FileManager::default()
            .set_output_path("dir_1")
            .set_file_name("file_1")
            .set_extension("dat")
            .build();

        let mut test_file_2 = FileManager::default()
            .set_output_path("dir_1")
            .set_file_name("file_2")
            .set_extension("csv")
            .build();

        let mut test_file_3 = FileManager::default()
            .set_output_path("dir_2")
            .set_file_name("file_3")
            .set_extension("dat")
            .build();

        let mut test_file_4 = FileManager::default()
            .set_output_path("dir_3")
            .set_file_name("file_4")
            .set_extension("txt")
            .build();

        let files = vec![
            &mut test_file_1,
            &mut test_file_2,
            &mut test_file_3,
            &mut test_file_4,
        ];

        assert_eq!(Ok(()), project_manager.initialize_output_files(files));

        let files = vec![
            &mut test_file_1,
            &mut test_file_2,
            &mut test_file_3,
            &mut test_file_4,
        ];

        assert_eq!(Ok(()), project_manager.initialize_output_files(files));

        // Verify that correct files were initialized
        assert!(Path::new("./test_overwrite/dir_1/file_1.dat").exists());

        assert!(Path::new("./test_overwrite/dir_1/file_2.dat").exists());

        assert!(Path::new("./test_overwrite/dir_2/file_3.dat").exists());

        assert!(Path::new("./test_overwrite/dir_3/file_4.dat").exists());

        // Verify final states of the FileManager
        assert_eq!(
            PathBuf::from("./test_overwrite/dir_1/file_1.dat")
                .canonicalize()
                .unwrap(),
            test_file_1.path()
        );

        assert_eq!(
            PathBuf::from("./test_overwrite/dir_1/file_2.dat")
                .canonicalize()
                .unwrap(),
            test_file_2.path()
        );

        assert_eq!(
            PathBuf::from("./test_overwrite/dir_2/file_3.dat")
                .canonicalize()
                .unwrap(),
            test_file_3.path()
        );

        assert_eq!(
            PathBuf::from("./test_overwrite/dir_3/file_4.dat")
                .canonicalize()
                .unwrap(),
            test_file_4.path()
        );

        // Delete test project directory tree
        if let Err(reason) = remove_dir_all("./test_overwrite/") {
            panic!(
                "Cannot remove project directory ./test_overwrite/: {:?}",
                reason.kind()
            );
        }
    }

    #[test]
    fn forbidden_overwrite() {
        // Setup test project directory tree
        let project_manager = ProjectManager {
            path: "test_panic".to_owned(),
            extension: "dat".to_owned(),
            overwrite_type: OverwriteType::Panic,
        };

        let mut test_file_1 = FileManager::default()
            .set_output_path("dir_1")
            .set_file_name("file_1")
            .set_extension("dat")
            .build();

        let mut test_file_2 = FileManager::default()
            .set_output_path("dir_1")
            .set_file_name("file_2")
            .set_extension("csv")
            .build();

        let mut test_file_3 = FileManager::default()
            .set_output_path("dir_2")
            .set_file_name("file_3")
            .set_extension("dat")
            .build();

        let mut test_file_4 = FileManager::default()
            .set_output_path("dir_3")
            .set_file_name("file_4")
            .set_extension("txt")
            .build();

        let files = vec![
            &mut test_file_1,
            &mut test_file_2,
            &mut test_file_3,
            &mut test_file_4,
        ];

        assert_eq!(Ok(()), project_manager.initialize_output_files(files));

        let mut test_file_1_copy = FileManager::default()
            .set_header("New file_1")
            .set_output_path("dir_1")
            .set_file_name("file_1")
            .set_extension("dat")
            .build();

        let files = vec![&mut test_file_1_copy];

        assert_eq!(
            Err(String::from(
                "Permission denied to overwrite existing output files."
            )),
            project_manager.initialize_output_files(files)
        );

        // Delete test project directory tree
        if let Err(reason) = remove_dir_all("./test_panic/") {
            panic!(
                "Cannot remove project directory ./test_panic/: {:?}",
                reason.kind()
            );
        }
    }

    #[test]
    fn archive_files() {
        // Setup test project directory tree
        let project_manager = ProjectManager {
            path: "test_archive".to_owned(),
            extension: "dat".to_owned(),
            overwrite_type: OverwriteType::Archive,
        };

        let mut test_file_1 = FileManager::default()
            .set_output_path("dir_1")
            .set_file_name("file_1")
            .set_extension("dat")
            .build();

        let mut test_file_2 = FileManager::default()
            .set_output_path("dir_1")
            .set_file_name("file_2")
            .set_extension("csv")
            .build();

        let mut test_file_3 = FileManager::default()
            .set_output_path("dir_2")
            .set_file_name("file_3")
            .set_extension("dat")
            .build();

        let mut test_file_4 = FileManager::default()
            .set_output_path("dir_3")
            .set_file_name("file_4")
            .set_extension("txt")
            .build();

        let files = vec![
            &mut test_file_1,
            &mut test_file_2,
            &mut test_file_3,
            &mut test_file_4,
        ];

        assert_eq!(Ok(()), project_manager.initialize_output_files(files));

        let mut test_file_1_copy = FileManager::default()
            .set_header("New file_1")
            .set_output_path("dir_1")
            .set_file_name("file_1")
            .set_extension("dat")
            .build();

        let files = vec![&mut test_file_1_copy];

        assert_eq!(Ok(()), project_manager.initialize_output_files(files));

        // Verify that correct files were initialized
        assert!(Path::new("./test_archive/dir_1/file_1.dat").exists());

        assert!(Path::new("./test_archive/dir_1/file_2.dat").exists());

        assert!(Path::new("./test_archive/dir_2/file_3.dat").exists());

        assert!(Path::new("./test_archive/dir_3/file_4.dat").exists());

        assert!(Path::new("./test_archive/archive/dir_1/file_1.dat").exists());

        // Verify final states of the FileManager
        assert_eq!(
            PathBuf::from("./test_archive/dir_1/file_1.dat")
                .canonicalize()
                .unwrap(),
            test_file_1.path()
        );

        assert_eq!(
            PathBuf::from("./test_archive/dir_1/file_2.dat")
                .canonicalize()
                .unwrap(),
            test_file_2.path()
        );

        assert_eq!(
            PathBuf::from("./test_archive/dir_2/file_3.dat")
                .canonicalize()
                .unwrap(),
            test_file_3.path()
        );

        assert_eq!(
            PathBuf::from("./test_archive/dir_3/file_4.dat")
                .canonicalize()
                .unwrap(),
            test_file_4.path()
        );

        assert_eq!(
            PathBuf::from("./test_archive/dir_1/file_1.dat")
                .canonicalize()
                .unwrap(),
            test_file_1_copy.path()
        );

        // Delete test project directory tree
        if let Err(reason) = remove_dir_all("./test_archive/") {
            panic!(
                "Cannot remove project directory ./test_archive/: {:?}",
                reason.kind()
            );
        }
    }

    #[test]
    fn ignore_files() {
        // Setup test project directory tree
        let project_manager = ProjectManager {
            path: "test_ignore".to_owned(),
            extension: "dat".to_owned(),
            overwrite_type: OverwriteType::Ignore,
        };

        let mut test_file_1 = FileManager::default()
            .set_output_path("dir_1")
            .set_file_name("file_1")
            .set_extension("dat")
            .build();

        let mut test_file_2 = FileManager::default()
            .set_output_path("dir_1")
            .set_file_name("file_2")
            .set_extension("csv")
            .build();

        let mut test_file_3 = FileManager::default()
            .set_output_path("dir_2")
            .set_file_name("file_3")
            .set_extension("dat")
            .build();

        let mut test_file_4 = FileManager::default()
            .set_output_path("dir_3")
            .set_file_name("file_4")
            .set_extension("txt")
            .build();

        let files = vec![
            &mut test_file_1,
            &mut test_file_2,
            &mut test_file_3,
            &mut test_file_4,
        ];

        assert_eq!(Ok(()), project_manager.initialize_output_files(files));

        let mut test_file_1_copy = FileManager::default()
            .set_header("New file_1")
            .set_output_path("dir_1")
            .set_file_name("file_1")
            .set_extension("dat")
            .build();

        let files = vec![&mut test_file_1_copy];

        assert_eq!(Ok(()), project_manager.initialize_output_files(files));

        // Verify that correct files were initialized
        assert!(Path::new("./test_ignore/dir_1/file_1.dat").exists());

        assert!(Path::new("./test_ignore/dir_1/file_2.dat").exists());

        assert!(Path::new("./test_ignore/dir_2/file_3.dat").exists());

        assert!(Path::new("./test_ignore/dir_3/file_4.dat").exists());

        // Verify final states of the FileManager
        assert_eq!(
            PathBuf::from("./test_ignore/dir_1/file_1.dat")
                .canonicalize()
                .unwrap(),
            test_file_1.path()
        );

        assert!(!test_file_1_copy.writable(), "{:?}", test_file_1_copy);

        assert_eq!(
            PathBuf::from("./test_ignore/dir_1/file_2.dat")
                .canonicalize()
                .unwrap(),
            test_file_2.path()
        );

        assert_eq!(
            PathBuf::from("./test_ignore/dir_2/file_3.dat")
                .canonicalize()
                .unwrap(),
            test_file_3.path()
        );

        assert_eq!(
            PathBuf::from("./test_ignore/dir_3/file_4.dat")
                .canonicalize()
                .unwrap(),
            test_file_4.path()
        );

        // Delete test project directory tree
        if let Err(reason) = remove_dir_all("./test_ignore/") {
            panic!(
                "Cannot remove project directory ./test_ignore/: {:?}",
                reason.kind()
            );
        }
    }

    #[test]
    fn define_builder() {
        let file = FileManager::default()
            .set_header("Some header")
            .set_project_path(".")
            .set_output_path("test")
            .set_file_name("test")
            .set_extension("dat")
            .build();

        assert_eq!(
            FileManager::Builder {
                header: Some(String::from("Some header")),
                project_path: Some(String::from(".")),
                output_path: Some(String::from("test")),
                name: Some(String::from("test")),
                extension: Some(String::from("dat")),
            },
            file
        );
    }

    #[test]
    fn initialize_file() {
        let mut file = FileManager::default()
            .set_header("Some header")
            .set_project_path(".")
            .set_file_name("test")
            .set_extension("dat")
            .build();

        file.to_initializer();

        assert_eq!(
            FileManager::Initializer {
                header: Some("Some header".to_owned()),
                path: PathBuf::from("./test.dat"),
            },
            file,
        );

        file.initialize_output();

        assert_eq!(
            FileManager::Writer {
                path: PathBuf::from("./test.dat").canonicalize().unwrap(),
                writable: true,
            },
            file,
        );
        remove_file(file.path()).expect("Could not delete test.dat file");
    }
}
