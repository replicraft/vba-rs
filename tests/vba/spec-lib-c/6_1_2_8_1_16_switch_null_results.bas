' INV: vbal.6.1.2.8.1.16, func.switch
' SOURCE: spec ms-vbal 6.1.2.8.1.16 Switch returns Null when no expression is True or the first True value is Null
' EXPECT: none=True
' EXPECT: nullvalue=True
Public Sub Main()
    Dim v As Variant
    v = Switch(False, "a", False, "b")
    Debug.Print "none=" & IsNull(v)
    v = Switch(False, "a", True, Null, True, "c")
    Debug.Print "nullvalue=" & IsNull(v)
End Sub
