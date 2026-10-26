' INV: vbal.5.4.5.3, stmt.seek, stmt.get
' SOURCE: spec ms-vbal 5.4.5.3 Seek on a Random file refers to a record
' EXPECT: 20
' EXPECT: 30
' GRANT: file-read
' GRANT: file-write
Public Sub Main()
    MkDir "/t"
    Dim n As Long, r As Long
    Open "/t/s.dat" For Random As #1 Len = 4
    n = 10: Put #1, 1, n
    n = 20: Put #1, 2, n
    n = 30: Put #1, 3, n
    Seek #1, 2
    Get #1, , r
    Debug.Print CStr(r)
    Get #1, , r
    Debug.Print CStr(r)
    Close #1
End Sub
