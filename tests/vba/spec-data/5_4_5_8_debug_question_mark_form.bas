' INV: vbal.5.4.5.8, stmt.print
' SOURCE: spec ms-vbal 5.4.5.8 revision 3.0 grammar: "?" as alternative to Print after Debug.
' XFAIL: wrong pending
' EXPECT: [q]
Public Sub Main()
    Debug.? "q"
End Sub
