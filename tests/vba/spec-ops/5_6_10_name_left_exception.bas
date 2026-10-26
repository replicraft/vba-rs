' INV: vbal.5.6.10, func.left
' SOURCE: spec ms-vbal 5.6.10 name Left with two arguments skips a parameterless user function of non-Variant type
' XFAIL: wrong pending
' EXPECT: ab
' EXPECT: 99
Public Function Left() As Long
    Left = 99
End Function
Public Sub Main()
    Debug.Print Left("abcdef", 2)
    Debug.Print CStr(Left)
End Sub
