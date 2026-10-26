' INV: vbal.6.1.2.11.1.29, func.replace
' SOURCE: spec ms-vbal 6.1.2.11.1.29 Replace omitted Compare uses the module Option Compare
' EXPECT: xxx
Option Compare Text
Public Sub Main()
    Debug.Print Replace("aAa", "a", "x")
End Sub
