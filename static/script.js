/**
 * delete task
 * @param {string} taskId - RustのDBに存在するuniqueID
 * @param {string} taskElementId - HTML上のタスクDOM
 */

async function deleteTask(taskId,taskElementId) {
    const taskItem = document.getElementById(taskElementId);
    if(!taskItem) {
        console.error(`エラー:HTML要素ID{taskElementId}のタスクが見つかりませんでした。`);
        return;
    }
    const taskNameSpan = taskItem.querySelector('.task-name');
    const taskName = taskNameSpan ? taskNameSpan.textContent:'不明なタスク';
    const completeButton = taskItem.querySelector('.complete-btn');

    if(completeButton) {
        completeButton.disabled = true;
        completeButton.textContent = '削除中...';
    }

    console.log(`taskID ${taskId} (${taskName}) の削除を試みます`);

    try {
        const response = await fetch(`/api/tasks/${taskId}` , {
            method : 'DELETE',
            headers:{
                'Content-Type' : 'application/json',
            }
        });

        if (response.ok) {
            // htmlから消す
            taskItem.remove();
            console.log(`task  "${taskName}" がUIから削除されました`);
            alert(`task "${taskName}" が完了しました`);
        } else {
            const errorText = await response.text();
            console.error(`タスク削除に失敗しました: ${response.status} ${response.statusText}`,errorText);
            alert(`task "${taskName}" の削除に失敗しました: ${errorText || '不明なエラー'}`);
            if(completeButton) {
                completeButton.disabled = false;
                completeButton.textContent = '完了';
            }
        }
    } catch (error) {
        console.error(`API呼び出し中にネットワークエラーが発生しました:`,error);
        alert(`ネットワークエラーが発生しました。タスク "${taskName}" を削除できませんでした。`);

        if(completeButton) {
            completeButton.disabled = false;
            completeButton.textContent = '完了';
        }
    }
}
