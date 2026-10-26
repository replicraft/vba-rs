' INV: vbal.5.4.5.8, stmt.print
' SOURCE: spec ms-vbal 5.4.5.8 revision 3.0 grammar lists only Debug./Me. forms of the print statement
' XFAIL: wrong VBA behaviour kept where MS-VBAL text differs: the file Print statement keeps its form without a Debug./Me. prefix (revision 3.0 grammar taken as additive)
' ERROR: load
' GRANT: file-read
' GRANT: file-write
Public Sub Main()
    Open "/p.txt" For Output As #1
    Print #1, "x"
End Sub
