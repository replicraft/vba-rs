' INV: vbal.6.1.2.7.1.11, func.rgb
' SOURCE: spec ms-vbal 6.1.2.7.1.11 RGB composes Blue*65536 + Green*256 + Red (formula max() is a typo for min)
' EXPECT: black=0
' EXPECT: mixed=197121
' EXPECT: red=255
' EXPECT: blue=16711680
' EXPECT: white=16777215
' EXPECT: type=Long
Public Sub Main()
    Debug.Print "black=" & CStr(RGB(0, 0, 0))
    Debug.Print "mixed=" & CStr(RGB(1, 2, 3))
    Debug.Print "red=" & CStr(RGB(255, 0, 0))
    Debug.Print "blue=" & CStr(RGB(0, 0, 255))
    Debug.Print "white=" & CStr(RGB(255, 255, 255))
    Debug.Print "type=" & TypeName(RGB(1, 2, 3))
End Sub
