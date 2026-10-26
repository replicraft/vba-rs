' INV: vbal.6.1.2.8.1.16, func.switch
' SOURCE: spec ms-vbal 6.1.2.8.1.16 Switch returns the value paired with the first True expression
' EXPECT: b
' EXPECT: c
' EXPECT: a
Public Sub Main()
    Debug.Print Switch(False, "a", True, "b", True, "c")
    Debug.Print Switch(False, "a", False, "b", True, "c")
    Debug.Print Switch(True, "a", True, "b")
End Sub
