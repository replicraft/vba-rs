' INV: vbal.6.1.2.2
' SOURCE: spec ms-vbal 6.1.2.2 Constants module constants usable as constant expressions
' EXPECT: a=-2147221503
' EXPECT: b=16711935
' EXPECT: c=3
Const A As Long = vbObjectError + 1
Const B As Long = vbRed Or vbBlue
Const C As Long = vbModal + vbCalHijri + vbHide
Public Sub Main()
    Debug.Print "a=" & CStr(A)
    Debug.Print "b=" & CStr(B)
    Debug.Print "c=" & CStr(C + 1)
End Sub
