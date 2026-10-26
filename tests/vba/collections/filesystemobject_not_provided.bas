' INV: obj.filesystemobject, obj.drive, obj.drives, obj.file, obj.files, obj.folder, obj.folders, obj.textstream
' SOURCE: doc https://learn.microsoft.com/en-us/office/vba/language/reference/user-interface-help/filesystemobject-object
' EXPECT: False
' GRANT: com-create
' XFAIL: unsupported Scripting.FileSystemObject is a COM library; the default host registers no classes (429)
Public Sub Main()
    Dim fso As Object
    Set fso = CreateObject("Scripting.FileSystemObject")
    Debug.Print fso.FileExists("/x")
End Sub
