' INV: func.getattr, stmt.setattr, stmt.open
' SOURCE: spec ms-vbal 6.1.2.5.2.7 SetAttr and 6.1.2.8.1.8 GetAttr
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
