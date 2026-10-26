' INV: vbal.3.3.2, lit.octal
' SOURCE: spec ms-vbal 3.3.2 number tokens, octal prefix forms & or &o or &O; hex digits case
' EXPECT: 15
' EXPECT: 15
' EXPECT: 15
' EXPECT: 255
' EXPECT: 255
' EXPECT: 255
Public Sub Main()
    Debug.Print CStr(&17)
    Debug.Print CStr(&o17)
    Debug.Print CStr(&O17)
    Debug.Print CStr(&hff)
    Debug.Print CStr(&HFF)
    Debug.Print CStr(&Hff)
End Sub
