// テスト用ファイル - 変更後のバージョン（破壊的変更）
// getUserByIdのシグネチャ変更
function getUserById(id, options) {
    return database.get(id, options);
}

// deleteUser関数を削除！（破壊的変更）

function updateUser(id, data) {
    return database.update(id, data);
}

module.exports = { getUserById, deleteUser, updateUser };