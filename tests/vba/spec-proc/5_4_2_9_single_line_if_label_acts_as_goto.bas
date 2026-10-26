' INV: vbal.5.4.2.9, stmt.if-then-else
' SOURCE: spec ms-vbal 5.4.2.9 a label right after Then behaves like a GoTo to that label
' XFAIL: wrong pending
' EXPECT: reached
Public Sub Main()
    If True Then Skip Else Debug.Print "no"
    Debug.Print "skipped"
Skip:
    Debug.Print "reached"
End Sub
