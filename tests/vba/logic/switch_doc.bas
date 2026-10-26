' INV: func.switch
' SOURCE: spec ms-vbal 6.1.2.8.1.16 Switch
' EXPECT: b
' EXPECT: y
Public Sub Main()
    Debug.Print CStr(Switch(False, "a", True, "b", True, "c"))
    Debug.Print CStr(Switch(1 = 2, "x", 2 = 2, "y"))
End Sub
