document.addEventListener('DOMContentLoaded', () => {
    const editButtons = document.querySelectorAll('.edit-toggle-button');
    const cancelEditButtons = document.querySelectorAll('.cancel-edit-button');
    const saveButtons = document.querySelectorAll('.save');
    
    editButtons.forEach(button => {
        button.addEventListener('click', (event) => {
            // クリックされたボタンの value (タスクID) を取得
            const itemId = event.target.value;

            // 取得したIDを使って、対応する編集フォームのIDを構築し、フォーム要素を取得
            const targetEditForm = document.getElementById(`edit-${itemId}`);
            const computedDisplay = window.getComputedStyle(targetEditForm).display;

            if(computedDisplay == 'none'){
                targetEditForm.style.display = 'block';
            } else {
                targetEditForm.style.display = 'none';
            }
        });
    });

    // --- 各「編集破棄」ボタンにイベントリスナーを設定 ---
    cancelEditButtons.forEach(button => {
        button.addEventListener('click', (event) => {
            // 「編集破棄」ボタンの親要素であるフォームを取得
            const parentForm = event.target.closest('.edit-form');
            if (parentForm) {
                parentForm.style.display = 'none'; // フォームを非表示にする
            }
        });
    });

    // 保存ボタンで同じこと
   saveButtons.forEach(button => {
       button.addEventListener('click' , (event) => {
           const parentForm = event.target.closest('.edit-form');
           if(parentForm){
               parentForm.style.display = 'none';
           }
       });
   });
});
