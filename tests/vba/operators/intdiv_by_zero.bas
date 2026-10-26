' INV: op.intdiv
' SOURCE: spec ms-vbal 5.6.9.3.6 integer division operator
' ERROR: 11
Public Sub Main()
    Debug.Print 5 \ 0
End Sub
