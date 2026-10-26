' INV: obj.filesystemobject
' SOURCE: unverified CopyFile, MoveFile, DeleteFile and their folder forms follow the Learn pages for the destination rules (a trailing separator or an existing folder means "into"); wildcards and the error numbers are assumptions
' EXPECT: x|x|False
' EXPECT: x
' EXPECT: 58
' EXPECT: x|True
' EXPECT: 0|0
' EXPECT: 53
' GRANT: com-create
' GRANT: file-read
' GRANT: file-write
Public Sub Main()
    Dim fs As Object
    Set fs = CreateObject("Scripting.FileSystemObject")
    fs.CreateFolder "/src"
    fs.CreateFolder "/dst"
    fs.CreateTextFile("/src/one.txt").Write "x"
    fs.CopyFile "/src/one.txt", "/dst/"
    fs.CopyFile "/src/one.txt", "/two.txt"
    Debug.Print fs.OpenTextFile("/dst/one.txt").ReadAll & "|" & fs.OpenTextFile("/two.txt").ReadAll & "|" & fs.FileExists("/src/zzz.txt")
    fs.MoveFile "/two.txt", "/dst"
    Debug.Print fs.OpenTextFile("/dst/two.txt").ReadAll
    On Error Resume Next
    fs.CopyFile "/src/one.txt", "/dst/one.txt", False
    Debug.Print Err.Number & ""
    On Error GoTo 0
    ' Folders.
    fs.CopyFolder "/src", "/copy"
    Debug.Print fs.OpenTextFile("/copy/one.txt").ReadAll & "|" & fs.FolderExists("/copy")
    fs.DeleteFile "/dst/*.txt"
    Debug.Print fs.GetFolder("/dst").Files.Count & "|" & 0
    fs.DeleteFolder "/copy"
    On Error Resume Next
    fs.DeleteFile "/nothing.txt"
    Debug.Print Err.Number & ""
End Sub
