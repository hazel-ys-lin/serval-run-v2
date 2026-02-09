use uuid::Uuid;

use serval_run::models::{
    Api, Collection, CreateApi, CreateCollection, CreateEnvironment, CreateProject, CreateReport,
    CreateUser, Environment, Project, Report, User,
};
use serval_run::repositories::{
    ApiRepository, CollectionRepository, EnvironmentRepository, ProjectRepository,
    ReportRepository, UserRepository,
};
use serval_run::services::AuthService;
use serval_run::state::AppState;

/// Authentication info for tests
#[allow(dead_code)]
pub struct TestAuth {
    pub user_id: Uuid,
    pub email: String,
    pub token: String,
}

impl TestAuth {
    /// Get the Authorization header value
    pub fn auth_header(&self) -> String {
        format!("Bearer {}", self.token)
    }
}

/// Factory for creating test data
pub struct Factory<'a> {
    state: &'a AppState,
}

#[allow(dead_code)]
impl<'a> Factory<'a> {
    pub fn new(state: &'a AppState) -> Self {
        Self { state }
    }

    /// Create a test user and return auth info
    pub async fn create_user(&self) -> TestAuth {
        let unique_id = Uuid::new_v4();
        let email = format!("test-{}@example.com", unique_id);
        let password = "TestPassword123!";

        let input = CreateUser {
            email: email.clone(),
            password: password.to_string(),
            name: format!("Test User {}", unique_id),
            job_title: Some("Tester".to_string()),
        };

        let password_hash = AuthService::hash_password(password).unwrap();
        let user = UserRepository::create(&self.state.db, &input, &password_hash)
            .await
            .unwrap();

        let token = AuthService::generate_token(user.id, &email, &self.state.config).unwrap();

        TestAuth {
            user_id: user.id,
            email,
            token,
        }
    }

    /// Create a test user with specific email
    pub async fn create_user_with_email(&self, email: &str, password: &str) -> User {
        let input = CreateUser {
            email: email.to_string(),
            password: password.to_string(),
            name: "Test User".to_string(),
            job_title: None,
        };

        let password_hash = AuthService::hash_password(password).unwrap();
        UserRepository::create(&self.state.db, &input, &password_hash)
            .await
            .unwrap()
    }

    /// Create a test project
    pub async fn create_project(&self, user_id: Uuid) -> Project {
        let input = CreateProject {
            name: format!("Test Project {}", Uuid::new_v4()),
            description: Some("Test project description".to_string()),
        };

        ProjectRepository::create(&self.state.db, user_id, &input)
            .await
            .unwrap()
    }

    /// Create a test project with specific name
    pub async fn create_project_with_name(&self, user_id: Uuid, name: &str) -> Project {
        let input = CreateProject {
            name: name.to_string(),
            description: None,
        };

        ProjectRepository::create(&self.state.db, user_id, &input)
            .await
            .unwrap()
    }

    /// Create a test collection
    pub async fn create_collection(&self, project_id: Uuid, user_id: Uuid) -> Collection {
        let input = CreateCollection {
            name: format!("Test Collection {}", Uuid::new_v4()),
            description: Some("Test collection description".to_string()),
        };

        CollectionRepository::create(&self.state.db, project_id, user_id, &input)
            .await
            .unwrap()
    }

    /// Create a test environment
    pub async fn create_environment(&self, project_id: Uuid, user_id: Uuid) -> Environment {
        let unique_id = Uuid::new_v4();
        let input = CreateEnvironment {
            title: format!("test-{}", unique_id),
            domain_name: format!("https://api.test-{}.example.com", unique_id),
        };

        EnvironmentRepository::create(&self.state.db, project_id, user_id, &input)
            .await
            .unwrap()
    }

    /// Create a test API
    pub async fn create_api(&self, collection_id: Uuid, user_id: Uuid) -> Api {
        let input = CreateApi {
            name: format!("Test API {}", Uuid::new_v4()),
            http_method: "GET".to_string(),
            endpoint: "/api/test".to_string(),
            severity: Some(1),
            description: Some("Test API description".to_string()),
        };

        ApiRepository::create(&self.state.db, collection_id, user_id, &input)
            .await
            .unwrap()
    }

    /// Create a test report
    pub async fn create_report(
        &self,
        project_id: Uuid,
        environment_id: Uuid,
        user_id: Uuid,
    ) -> Report {
        let input = CreateReport {
            environment_id,
            collection_id: None,
            report_level: 2, // project level
            report_type: Some("test".to_string()),
        };

        ReportRepository::create(&self.state.db, project_id, user_id, &input)
            .await
            .unwrap()
    }

    /// Create a full test hierarchy: user -> project -> collection -> environment
    pub async fn create_hierarchy(&self) -> TestHierarchy {
        let auth = self.create_user().await;
        let project = self.create_project(auth.user_id).await;
        let collection = self.create_collection(project.id, auth.user_id).await;
        let environment = self.create_environment(project.id, auth.user_id).await;

        TestHierarchy {
            auth,
            project,
            collection,
            environment,
        }
    }
}

/// Complete test data hierarchy
#[allow(dead_code)]
pub struct TestHierarchy {
    pub auth: TestAuth,
    pub project: Project,
    pub collection: Collection,
    pub environment: Environment,
}
