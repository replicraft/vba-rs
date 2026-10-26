' INV: vbal.5.4.5.1, stmt.open
' SOURCE: spec ms-vbal 5.4.5.1 Append requires Read Write or Write
' XFAIL: wrong pending
' ERROR: load
' GRANT: file-read
' GRANT: file-write
Public Sub Main()
    Open "/o.txt" For Append Access Read As #1
End Sub
