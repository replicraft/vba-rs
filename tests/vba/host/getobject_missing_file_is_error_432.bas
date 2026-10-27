' INV: func.getobject, vbal.6.1.2.8.1.9
' SOURCE: derived a pathname that names no file cannot be bound: error 432 (File name or class name not found during Automation operation)
' METHOD: by hand; the in-memory file system holds no such file
' GRANT: file-read
' EXPECT:  432 
' EXPECT: True
Public Sub Main()
    Dim o As Object
    On Error Resume Next
    Set o = GetObject("missing-document.doc")
    Debug.Print Err.Number
    Debug.Print o Is Nothing
End Sub
