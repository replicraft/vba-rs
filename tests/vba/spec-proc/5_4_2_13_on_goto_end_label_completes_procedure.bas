' INV: vbal.5.4.2.13
' SOURCE: spec ms-vbal 5.4.2.13 branching to the end-label completes the procedure
' EXPECT: caller-continues
Sub S()
    On 2 GoTo L1, Fin
L1: Debug.Print "no"
Fin:
End Sub
Public Sub Main()
    S
    Debug.Print "caller-continues"
End Sub
