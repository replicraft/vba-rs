' INV: stmt.open
' SOURCE: spec ms-vbal 5.4.5.1 Open Statement error 55 file already open
' ERROR: 55
' GRANT: file-read
' GRANT: file-write
Public Sub Main()
    MkDir "/t"
    Open "/t/x" For Output As #1
    Open "/t/y" For Output As #1
End Sub
