' INV: vbal.5.4.5.8, stmt.print
' SOURCE: spec ms-vbal 5.4.5.8 revision 3.0 grammar: unopened file number is error 52
' XFAIL: wrong pending
' ERROR: 52
' GRANT: file-read
' GRANT: file-write
Public Sub Main()
    Debug.Print #9, "x"
End Sub
