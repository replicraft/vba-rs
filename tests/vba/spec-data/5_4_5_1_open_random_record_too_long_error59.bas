' INV: vbal.5.4.5.1, stmt.open, stmt.put
' SOURCE: spec ms-vbal 5.4.5.11 more bytes than Len is error 59 (Bad record length)
' ERROR: 59
' GRANT: file-read
' GRANT: file-write
Public Sub Main()
    MkDir "/t"
    Dim a As Long
    a = 5
    Open "/t/rl.dat" For Random As #1 Len = 2
    Put #1, 1, a
End Sub
