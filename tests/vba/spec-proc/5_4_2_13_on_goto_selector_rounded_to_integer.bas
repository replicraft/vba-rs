' INV: vbal.5.4.2.13
' SOURCE: spec ms-vbal 5.4.2.13 the selector is Let-coerced to Integer before selecting
' EXPECT: two
Public Sub Main()
    On 1.6 GoTo L1, L2, L3
    Debug.Print "fell"
    Exit Sub
L1: Debug.Print "one": Exit Sub
L2: Debug.Print "two": Exit Sub
L3: Debug.Print "three"
End Sub
