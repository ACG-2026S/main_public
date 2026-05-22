using UnityEngine;

public class CircleMotion : MonoBehaviour
{
     public Material targetMaterial;

    void Update()
    {
        float radius = 0.3f;
        float t = Time.time * 2.0f * Mathf.PI;
        float x = radius * Mathf.Cos(t);
        float z = radius * Mathf.Sin(t);
        Vector3 p = new Vector3(x, 0.8f, z);
        transform.position = p;
        Material mat = targetMaterial;
        mat.SetVector(
            "_LightPos",
            new Vector4(p.x, p.y, p.z, 1.0f)
        );
    }
}
