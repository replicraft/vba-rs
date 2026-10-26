' INV: stmt.on-gosub-on-goto
' SOURCE: spec ms-vbal 5.4.2.13 On GoTo statement index above the label count
' EXPECT: fell
Public Sub Main()
    On 7 GoTo A, B
    Debug.Print "fell"
    Exit Sub
A:
B:
    Debug.Print "no"
End Sub
