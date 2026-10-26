' INV: vbal.5.4.5.1.1, stmt.open
' SOURCE: spec ms-vbal 5.4.5.1.1 file number above 511 is out of range
' ERROR: 52
' GRANT: file-read
' GRANT: file-write
Public Sub Main()
    MkDir "/t"
    Open "/t/f" For Output As #512
End Sub
