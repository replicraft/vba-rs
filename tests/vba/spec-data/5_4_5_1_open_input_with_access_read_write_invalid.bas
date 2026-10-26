' INV: vbal.5.4.5.1, stmt.open
' SOURCE: spec ms-vbal 5.4.5.1 Input requires access to be only Read
' XFAIL: wrong VBA behaviour kept where MS-VBAL text differs: incompatible Open mode/access pairs fail at run time, not at analysis
' ERROR: load
' GRANT: file-read
' GRANT: file-write
Public Sub Main()
    Open "/o.txt" For Input Access Read Write As #1
End Sub
