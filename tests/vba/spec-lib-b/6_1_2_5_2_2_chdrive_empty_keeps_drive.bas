' INV: vbal.6.1.2.5.2.2, stmt.chdrive
' SOURCE: spec ms-vbal 6.1.2.5.2.2 ChDrive with zero-length string leaves current drive unchanged
' EXPECT: same=True
' GRANT: file-read
' GRANT: file-write
Public Sub Main()
    Dim before As String
    before = CurDir
    ChDrive ""
    Debug.Print "same=" & CStr(CurDir = before)
End Sub
