' INV: obj.filesystemobject
' SOURCE: doc https://learn.microsoft.com/en-us/office/vba/language/reference/user-interface-help/filesystemobject-object
' EXPECT: False
' GRANT: com-create
' GRANT: file-read
Public Sub Main()
    Dim fso As Object
    Set fso = CreateObject("Scripting.FileSystemObject")
    Debug.Print fso.FileExists("/x")
End Sub
