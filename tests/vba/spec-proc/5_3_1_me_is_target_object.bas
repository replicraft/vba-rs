' INV: vbal.5.3.1
' SOURCE: spec ms-vbal 5.3.1 Me refers to the target object of the active invocation
' EXPECT: True
' EXPECT: False
'#MODULE Cls class
Public Function Self() As Cls
    Set Self = Me
End Function
'#MODULE Module1 standard
Public Sub Main()
    Dim a As New Cls
    Dim b As New Cls
    Debug.Print CStr(a.Self() Is a)
    Debug.Print CStr(a.Self() Is b)
End Sub
