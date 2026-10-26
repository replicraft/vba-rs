' INV: vbal.5.2.1.2
' SOURCE: spec ms-vbal 5.2.1.2 Option Base in a class module declaration section
' EXPECT: cls=1
' EXPECT: main=0
Public Sub Main()
    Dim c As New Arr
    Debug.Print "cls=" & CStr(c.Lb())
    Dim a(2) As Long
    Debug.Print "main=" & CStr(LBound(a))
End Sub
'#MODULE Arr class
Option Base 1
Public Function Lb() As Long
    Dim a(2) As Long
    Lb = LBound(a)
End Function
