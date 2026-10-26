' INV: vbal.5.6.2.2, vbal.5.6.6
' SOURCE: spec ms-vbal 5.6.2.2 Evaluation to a simple data value, an object without a public default member raises error 438
' XFAIL: wrong not implemented: error 438 for (v) when v holds an object without a default member
' ERROR: 438
'#MODULE Plain class
Public Sub Hello()
End Sub
'#MODULE Module1 standard
Public Sub Main()
    Dim v As Variant
    Dim x As Variant
    Set v = New Plain
    x = (v)
    Debug.Print "not reached"
End Sub
