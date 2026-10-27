' INV: func.objptr
' SOURCE: derived ObjPtr identifies an object: equal for two references to one object, different for different objects, 0 for Nothing; real VBA returns an address, this implementation an opaque token, so the value is never printed
' METHOD: by hand; the identity relations are what code that keys on ObjPtr relies on
' EXPECT: True
' EXPECT: True
' EXPECT: True
' EXPECT: True
' EXPECT: True
Public Sub Main()
    Dim a As New Collection
    Dim b As Object
    Dim c As New Collection
    Dim n As Object
    Set b = a
    Debug.Print ObjPtr(a) = ObjPtr(b)
    Debug.Print ObjPtr(a) <> ObjPtr(c)
    Debug.Print ObjPtr(a) <> 0
    Debug.Print ObjPtr(n) = 0
    Debug.Print ObjPtr(a) = ObjPtr(a)
End Sub
