' INV: func.getattr, stmt.setattr, stmt.open
' SOURCE: doc https://learn.microsoft.com/en-us/office/vba/language/reference/user-interface-help/setattr-statement
' EXPECT:  1 
' EXPECT:  0 
' GRANT: file-read
' GRANT: file-write
Public Sub Main()
    Open "/a.txt" For Output As #1: Close #1
    SetAttr "/a.txt", vbReadOnly
    Debug.Print GetAttr("/a.txt") And vbReadOnly
    SetAttr "/a.txt", vbNormal
    Debug.Print GetAttr("/a.txt") And vbReadOnly
End Sub
