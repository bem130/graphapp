export function update(data) {
    console.log(data);
    if (!myEditor) {
        defaultCode = data;
    }
    else {
        myEditor.setValue(data);
    }
    return true;
}