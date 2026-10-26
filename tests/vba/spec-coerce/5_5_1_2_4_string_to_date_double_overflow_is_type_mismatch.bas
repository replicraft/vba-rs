' INV: vbal.5.5.1.2.4, expr.implicit-conv
' SOURCE: spec ms-vbal 5.5.1.2.4 Let-coercion to and from String, a number string overflowing Double on the way to Date raises 13 not 6
' ERROR: 13
Public Sub Main()
    Dim d As Date
    d = "1E400"
    Debug.Print "not reached"
End Sub
