' INV: vbal.3.3.2, lit.hex
' SOURCE: spec ms-vbal 3.3.2 number tokens, hex literal with % and & suffix: value by range
' EXPECT: 32767
' EXPECT: -32768
' EXPECT: -1
' EXPECT: 32768
' EXPECT: 65535
' EXPECT: -1
' EXPECT: -2147483648
' EXPECT: Integer
Public Sub Main()
    Debug.Print CStr(&H7FFF%)
    Debug.Print CStr(&H8000%)
    Debug.Print CStr(&HFFFF%)
    Debug.Print CStr(&H8000&)
    Debug.Print CStr(&HFFFF&)
    Debug.Print CStr(&HFFFFFFFF&)
    Debug.Print CStr(&H80000000&)
    Debug.Print TypeName(&HFFFF%)
End Sub
