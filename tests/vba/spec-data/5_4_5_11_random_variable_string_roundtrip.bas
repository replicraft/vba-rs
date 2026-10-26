' INV: vbal.5.4.5.11, stmt.put, stmt.get
' SOURCE: spec ms-vbal 5.4.5.12 Random: String has a two byte length prefix and round-trips
' EXPECT: hello|ab
' GRANT: file-read
' GRANT: file-write
Public Sub Main()
    MkDir "/t"
    Dim a As String, b As String, r1 As String, r2 As String
    a = "hello": b = "ab"
    Open "/t/g.dat" For Random As #1 Len = 20
    Put #1, 1, a
    Put #1, 2, b
    Get #1, 1, r1
    Get #1, 2, r2
    Close #1
    Debug.Print r1 & "|" & r2
End Sub
