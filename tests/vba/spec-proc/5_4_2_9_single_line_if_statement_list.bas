' INV: vbal.5.4.2.9, stmt.if-then-else
' SOURCE: spec ms-vbal 5.4.2.9 the colon-separated list after Then runs entirely or not at all
' EXPECT: t1
' EXPECT: t2
' EXPECT: e1
' EXPECT: e2
' EXPECT: end
Public Sub Main()
    If True Then Debug.Print "t1": Debug.Print "t2" Else Debug.Print "no"
    If False Then Debug.Print "no" Else Debug.Print "e1": Debug.Print "e2"
    If False Then Debug.Print "no1": Debug.Print "no2"
    Debug.Print "end"
End Sub
