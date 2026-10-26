' INV: vbal.5.4.2.9, stmt.if-then-else
' SOURCE: spec ms-vbal 5.4.2.9 first Else pairs with the nearest If, later ones with preceding Ifs
' EXPECT: a-true-b-false:inner-else
' EXPECT: a-false:outer-else
' EXPECT: a-true-b-true:inner-then
Sub T(ByVal a As Boolean, ByVal b As Boolean)
    If a Then If b Then Debug.Print "a-true-b-true:inner-then" Else Debug.Print "a-true-b-false:inner-else" Else Debug.Print "a-false:outer-else"
End Sub
Public Sub Main()
    T True, False
    T False, True
    T True, True
End Sub
