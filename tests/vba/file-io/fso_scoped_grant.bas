' INV: obj.filesystemobject
' SOURCE: derived grants scoped to a path prefix apply to the FileSystemObject exactly as to the file statements: inside the prefix it works, outside it is error 70
' EXPECT: True
' EXPECT: 70
' EXPECT: 70
' GRANT: com-create
' GRANT: file-read=/ok
' GRANT: file-write=/ok
Public Sub Main()
    Dim fs As Object
    Set fs = CreateObject("Scripting.FileSystemObject")
    fs.CreateFolder "/ok"
    fs.CreateTextFile("/ok/a.txt").Close
    Debug.Print fs.FileExists("/ok/a.txt")
    On Error Resume Next
    fs.CreateTextFile "/elsewhere.txt"
    Debug.Print Err.Number & ""
    Err.Clear
    Debug.Print fs.FileExists("/ok/../elsewhere.txt")
    Debug.Print Err.Number & ""
End Sub
