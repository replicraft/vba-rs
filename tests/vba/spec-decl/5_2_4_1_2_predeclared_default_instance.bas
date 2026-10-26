' INV: vbal.5.2.4.1.2
' SOURCE: spec ms-vbal 5.2.4.1.2 VB_PredeclaredId gives a default instance variable named as the class
' EXPECT: 5
' EXPECT: 5
' EXPECT: 0
Public Sub Main()
    Counter.n = 5
    Debug.Print CStr(Counter.n)
    Dim other As New Counter
    Debug.Print CStr(Counter.n)
    Debug.Print CStr(other.n)
End Sub
'#MODULE Counter class
Attribute VB_PredeclaredId = True
Public n As Long
