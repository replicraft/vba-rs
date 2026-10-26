' INV: vbal.6.1.2.5.2.7, stmt.setattr
' SOURCE: spec ms-vbal 6.1.2.5.2.7 SetAttr sets attribute information; error on an open file
' EXPECT: readonly=True
' EXPECT: raised=True
' GRANT: file-read
' GRANT: file-write
Public Sub Main()
    Dim f As Integer
    f = FreeFile
    Open "/sa.txt" For Output As #f
    Close #f
    SetAttr "/sa.txt", vbReadOnly
    Debug.Print "readonly=" & CStr((GetAttr("/sa.txt") And vbReadOnly) = vbReadOnly)
    SetAttr "/sa.txt", vbNormal
    Open "/sa.txt" For Output As #f
    On Error Resume Next
    SetAttr "/sa.txt", vbReadOnly
    Debug.Print "raised=" & CStr(Err.Number <> 0)
    On Error GoTo 0
    Close #f
End Sub
