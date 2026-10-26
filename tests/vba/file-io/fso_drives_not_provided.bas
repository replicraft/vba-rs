' INV: obj.drive, obj.drives
' SOURCE: doc https://learn.microsoft.com/en-us/office/vba/language/reference/user-interface-help/filesystemobject-object
' EXPECT: True
' GRANT: com-create
' XFAIL: unsupported FileSystemObject.Drives, GetDrive and DriveExists are not provided: a virtual file system has no drives to describe (error 438)
Public Sub Main()
    Dim fs As Object
    Set fs = CreateObject("Scripting.FileSystemObject")
    Debug.Print fs.Drives.Count > 0
End Sub
