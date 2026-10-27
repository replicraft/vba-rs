' INV: expr.addressof, vbal.5.6.16.8
' SOURCE: doc https://learn.microsoft.com/en-us/office/vba/language/reference/user-interface-help/addressof-operator
' EXPECT: True
' EXPECT: True
Function Cb() As Long
    Cb = 1
End Function
Public Sub Main()
    Dim p As LongPtr
    p = AddressOf Cb
    Debug.Print p <> 0
    Debug.Print p = AddressOf Cb
End Sub
