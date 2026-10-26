' INV: func.cbyte
' SOURCE: spec ms-vbal 5.5.1.2.1 numeric Let-coercion with Banker's rounding
' EXPECT: 126
Public Sub Main()
    Debug.Print CStr(CByte(125.5678))
End Sub
