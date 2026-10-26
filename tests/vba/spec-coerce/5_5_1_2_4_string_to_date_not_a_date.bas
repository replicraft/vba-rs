' INV: vbal.5.5.1.2.4, expr.implicit-conv
' SOURCE: spec ms-vbal 5.5.1.2.4 Let-coercion to and from String, text that is neither date, time nor number is Type mismatch
' ERROR: 13
Public Sub Main()
    Dim d As Date
    d = "hello"
    Debug.Print "not reached"
End Sub
