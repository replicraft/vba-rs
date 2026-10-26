' INV: vbal.5.2.1.2
' SOURCE: spec ms-vbal 5.2.1.2 Option Base is per module
' EXPECT: here=0
' EXPECT: there=1
Public Sub Main()
    Dim a(3) As Long
    Debug.Print "here=" & CStr(LBound(a))
    Debug.Print "there=" & CStr(Other.Lb())
End Sub
'#MODULE Other standard
Option Base 1
Public Function Lb() As Long
    Dim a(3) As Long
    Lb = LBound(a)
End Function
