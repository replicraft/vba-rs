' INV: vbal.5.5.1.2.4, expr.implicit-conv
' SOURCE: spec ms-vbal 5.5.1.2.4 Let-coercion to and from String, the empty string cannot be interpreted as a number
' ERROR: 13
Public Sub Main()
    Dim x As Double
    Dim s As String
    s = ""
    x = s
    Debug.Print "not reached"
End Sub
