CREATE INDEX IF NOT EXISTS task_id ON task_call_metrics USING btree (task_id);
CREATE INDEX IF NOT EXISTS employee_quality_score ON task_call_metrics USING btree (employee_quality_score);
CREATE INDEX IF NOT EXISTS script_score ON task_call_metrics USING btree (script_score);

CREATE INDEX IF NOT EXISTS project_id ON task using btree (project_id);
CREATE INDEX IF NOT EXISTS call_metadata_id ON task USING btree (call_metadata_id);

CREATE INDEX IF NOT EXISTS call_id ON call_metadata USING btree (call_id);
CREATE INDEX IF NOT EXISTS performed_at ON call_metadata USING BRIN (performed_at);
CREATE INDEX IF NOT EXISTS uploaded_at ON call_metadata USING BRIN (uploaded_at);
CREATE INDEX IF NOT EXISTS duration ON call_metadata USING btree (duration);
CREATE INDEX IF NOT EXISTS file_url ON call_metadata USING btree (file_url);
CREATE INDEX IF NOT EXISTS file_name ON call_metadata USING btree (file_name);

CREATE INDEX IF NOT EXISTS project_id ON settings using btree (project_id);
CREATE INDEX IF NOT EXISTS settings_id ON settings_item using btree (settings_id);
CREATE INDEX IF NOT EXISTS settings_item_id ON settings_dict_item using btree (settings_item_id);
CREATE INDEX IF NOT EXISTS dictionary_id ON settings_dict_item using btree (dictionary_id);

CREATE INDEX IF NOT EXISTS task_id ON task_to_dict USING btree (task_id);
CREATE INDEX IF NOT EXISTS dictionary_id ON task_to_dict USING btree (dictionary_id);

CREATE INDEX IF NOT EXISTS dictionary_id ON phrase USING btree (dictionary_id);
