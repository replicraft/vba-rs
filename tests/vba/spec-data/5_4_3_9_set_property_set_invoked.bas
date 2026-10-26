' INV: vbal.5.4.3.9, stmt.set
' SOURCE: spec ms-vbal 5.4.3.9 Set on a property invokes the Property Set
' EXPECT: set called
' EXPECT: 8
'#MODULE C1 class
Public V As Long
'#MODULE C2 class
Private m As C1
Public Property Set Child(ByVal x As C1)
    Debug.Print "set called"
    Set m = x
End Property
Public Property Get Child() As C1
    Set Child = m
End Property
'#MODULE Module1 standard
Public Sub Main()
    Dim p As New C2
    Dim k As New C1
    k.V = 8
    Set p.Child = k
    Debug.Print CStr(p.Child.V)
End Sub
