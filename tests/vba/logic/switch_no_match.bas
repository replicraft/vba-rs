' INV: func.switch, lit.null
' SOURCE: spec ms-vbal 6.1.2.8.1.16 Switch
' EXPECT: True
Public Sub Main()
    Debug.Print IsNull(Switch(False, 1, False, 2))
End Sub
