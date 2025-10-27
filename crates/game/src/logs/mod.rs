#[cfg(feature = "m2_logs")]
pub mod m2;

#[cfg(not(feature = "m2_logs"))]
pub mod m2 {
    use anyhow::Result;
    use repro::Command;

    #[derive(Debug)]
    pub struct SpawnBudgetLog<'a> {
        pub tick: u32,
        pub link_id: &'a str,
        pub pp: i32,
        pub weather: &'a str,
        pub budget: Budget<'a>,
    }

    #[derive(Debug)]
    pub struct Budget<'a> {
        pub enemies: u32,
        pub obstacles: u32,
        pub note: Option<&'a str>,
    }

    #[derive(Debug)]
    pub struct MissionResultLog<'a> {
        pub name: &'a str,
        pub outcome: &'a str,
        pub pp_delta: i16,
        pub basis_bp_overlay: i16,
    }

    #[derive(Debug)]
    pub struct PostLegSummaryLog {
        pub danger_delta: i32,
        pub applied_pp_delta: i16,
        pub applied_basis_overlay: i16,
        pub di_bp_after: i16,
        pub basis_bp_after: i16,
    }

    #[derive(Debug)]
    pub struct ReplayMismatchLog {
        pub tick: u32,
        pub expected: Option<Command>,
        pub got: Option<Command>,
    }

    pub fn write_spawn_budget(_log: SpawnBudgetLog<'_>) -> Result<()> {
        Ok(())
    }

    pub fn write_mission_result(_log: MissionResultLog<'_>) -> Result<()> {
        Ok(())
    }

    pub fn write_post_leg_summary(_log: PostLegSummaryLog) -> Result<()> {
        Ok(())
    }

    pub fn write_replay_mismatch(_log: ReplayMismatchLog) -> Result<()> {
        Ok(())
    }
}
