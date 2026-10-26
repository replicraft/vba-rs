' INV: obj.filesystemobject
' SOURCE: unverified path text functions follow their Learn pages (BuildPath inserts a separator only if necessary); the separator chosen for a forward-slash path and the handling of a root are assumptions
' EXPECT: c:\dir\a.txt
' EXPECT: c:\dir\a.txt
' EXPECT: /dir/a.txt
' EXPECT: a.txt|a|txt
' EXPECT: c:\dir|c:
' EXPECT: .gitignore||gitignore
' GRANT: com-create
Public Sub Main()
    Dim fs As Object
    Set fs = CreateObject("Scripting.FileSystemObject")
    Debug.Print fs.BuildPath("c:\dir", "a.txt")
    Debug.Print fs.BuildPath("c:\dir\", "a.txt")
    Debug.Print fs.BuildPath("/dir", "a.txt")
    Debug.Print fs.GetFileName("c:\dir\a.txt") & "|" & fs.GetBaseName("c:\dir\a.txt") & "|" & fs.GetExtensionName("c:\dir\a.txt")
    Debug.Print fs.GetParentFolderName("c:\dir\a.txt") & "|" & fs.GetDriveName("c:\dir\a.txt")
    Debug.Print fs.GetFileName(".gitignore") & "|" & fs.GetBaseName(".gitignore") & "|" & fs.GetExtensionName(".gitignore")
End Sub
