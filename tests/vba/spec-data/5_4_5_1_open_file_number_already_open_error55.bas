' INV: vbal.5.4.5.1, stmt.open
' SOURCE: spec ms-vbal 5.4.5.1 file number already open is error 55
' ERROR: 55
' GRANT: file-read
' GRANT: file-write
Public Sub Main()
    MkDir "/t"
    Open "/t/x1" For Output As #3
    Open "/t/x2" For Output As #3
End Sub
