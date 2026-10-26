' INV: obj.filesystemobject, obj.textstream
' SOURCE: unverified the pages say only that an error occurs; the numbers 53 (file not found), 76 (path not found), 58 (file exists), 62 (input past end) and 54 (bad mode) are assumptions
' EXPECT: 53
' EXPECT: 76
' EXPECT: 58
' EXPECT: 62
' EXPECT: 54
' EXPECT: 62
' EXPECT: 54
' GRANT: com-create
' GRANT: file-read
' GRANT: file-write
Public Sub Main()
    Dim fs As Object, t As Object
    Set fs = CreateObject("Scripting.FileSystemObject")
    On Error Resume Next
    fs.OpenTextFile "/missing.txt"
    Debug.Print Err.Number & ""
    Err.Clear
    fs.OpenTextFile "/nodir/x.txt", 2, True
    Debug.Print Err.Number & ""
    Err.Clear
    fs.CreateTextFile("/e.txt").Close
    fs.CreateTextFile "/e.txt", False
    Debug.Print Err.Number & ""
    Err.Clear
    Set t = fs.OpenTextFile("/e.txt")
    t.ReadAll
    Debug.Print Err.Number & ""
    Err.Clear
    t.Write "x"
    Debug.Print Err.Number & ""
    Err.Clear
    t.ReadLine
    Debug.Print Err.Number & ""
    Err.Clear
    t.Close
    t.ReadAll
    Debug.Print Err.Number & ""
End Sub
