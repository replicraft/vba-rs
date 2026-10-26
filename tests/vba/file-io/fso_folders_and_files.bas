' INV: obj.filesystemobject, obj.folder, obj.folders, obj.file, obj.files
' SOURCE: unverified folder and file objects follow the object model on the Learn pages; Type strings, the single timestamp and the order of the collections are assumptions
' EXPECT: True|False
' EXPECT: data|/data
' EXPECT: 2|1
' EXPECT: a.txt b.txt
' EXPECT: sub
' EXPECT: 5|TXT File|a.txt|/data/a.txt
' EXPECT: 5|/data
' EXPECT: True
' EXPECT: False
' GRANT: com-create
' GRANT: file-read
' GRANT: file-write
Public Sub Main()
    Dim fs As Object, fo As Object, f As Object, s As String, x As Object
    Set fs = CreateObject("Scripting.FileSystemObject")
    fs.CreateFolder "/data"
    fs.CreateFolder "/data/sub"
    Debug.Print fs.FolderExists("/data") & "|" & fs.FileExists("/data")
    fs.CreateTextFile("/data/a.txt").Write "hello"
    fs.CreateTextFile("/data/b.txt").Close
    Set fo = fs.GetFolder("/data")
    Debug.Print fo.Name & "|" & fo.Path
    Debug.Print fo.Files.Count & "|" & fo.SubFolders.Count
    For Each f In fo.Files
        s = s & IIf(s = "", "", " ") & f.Name
    Next f
    Debug.Print s
    For Each x In fo.SubFolders
        Debug.Print x.Name
    Next x
    Set f = fs.GetFile("/data/a.txt")
    Debug.Print f.Size & "|" & f.Type & "|" & f.Name & "|" & f.Path
    Debug.Print fo.Size & "|" & f.ParentFolder.Path
    fo.Delete
    Debug.Print fs.FileExists("/data/a.txt") = False
    Debug.Print fs.FolderExists("/data")
End Sub
